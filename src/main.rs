use url::Url;
use clap::{Parser, Subcommand};

use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(name = "Automa")]
#[command(author = "João Scheuermann <jvitor.sche@gmail.com>")]
#[command(version = "0.1.0")]
#[command(about = "Executes scripts and help automate repetitive tasks with js", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Runs a script from an url or saved locally
    Run {
        command_collection: String,
        command_name: String,
    },

    /// Adds a command to be used later
    Add {
        command_collection: String,
        command_name: String,
        command_url: String,
    },
}

fn is_valid_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[allow(clippy::needless_pass_by_value)] // this function should follow the callback type
fn log_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    let message = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);

    println!("Logged: {}", message);
}

fn execute (command: String) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize V8.
    let platform = v8::new_default_platform(0, false).make_shared();

    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    {
        let mut isolate = v8::Isolate::new(v8::CreateParams::default());
        let mut scope = v8::HandleScope::new(&mut isolate);

        // Create a console object
        let global = v8::ObjectTemplate::new(&mut scope);

        global.set(
            v8::String::new(&mut scope, "log").unwrap().into(),
            v8::FunctionTemplate::new(&mut scope, log_callback).into(),
        );

        let context = v8::Context::new_from_template(&mut scope, global);
        let mut context_scope = v8::ContextScope::new(&mut scope, context);

        // Create a string containing the JavaScript source code.
        let code = v8::String::new(&mut context_scope, command.as_str()).unwrap();

        // Compile the source code.
        let script = v8::Script::compile(&mut context_scope, code, None).unwrap();
        // Run the script to get the result.
        let result = script.run(&mut context_scope).unwrap();

        // Convert the result to a string and print it.
        let result = result.to_string(&mut context_scope).unwrap();
        println!("{}", result.to_rust_string_lossy(&mut context_scope));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Run {
            command_collection,
            command_name,
        }) => {
            // Check for an existing path on the commands folder with the given collection and name
            let mut path = PathBuf::new();

            path.push("commands");
            path.push(command_collection);
            path.push(command_name);

            if !path.exists() {
                return Err("Command not found".into());
            }

            // Read the file
            let mut file = File::open(path.join("index.js"))?;
            let mut contents = String::new();

            file.read_to_string(&mut contents)?;

            println!("Running command: {} from collection: {}", command_name, command_collection);

            execute(contents)?;
        }

        Some(Commands::Add {
            command_collection,
            command_name,
            command_url,
        }) => {
            if !is_valid_url(command_url) {
                return Err("Invalid URL".into());
            }

            let response = reqwest::get(command_url)
                .await?;

            match response.status().as_u16() {
                200 => {
                    let body = response.bytes().await?;
                    let mut path = PathBuf::new();

                    path.push("commands");
                    path.push(command_collection);
                    path.push(command_name);

                    std::fs::create_dir_all(path.clone())?;

                    let mut file = File::create(path.join("index.js"))?;

                    file.write_all(&body)?;

                    println!("Command added successfully!");
                }

                404 => {
                    return Err("404: Not Found".into());
                }

                _ => {
                    println!("Unexpected status code: {}", response.status());
                }
            }
        }

        None => {}
    }

    Ok(())
}