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

struct Command {
    command_collection: String,
    command_name: String,
    command_url: String,
    command_data: String,
}

fn is_valid_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut commands: Vec<Command> = vec![];

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