// TODO: implementar console.log
// TODO: implementar prompt
// TODO: implementar localStorage
// TODO: implementar exec/spawn
// TODO: implementar check da versão do gist ou do arquivo salvo. Se for diferente, baixar novamente.
// TODO: refatorar o comando 'add' para adicionar multiplas fontes de dados ex: automa add --from=gist --url=... --token=... (opcional, caso seja um gist privado)
// TODO: implementar login no git
// TODO: implementar backup dos comandos no gist do usuário
// TODO: implementar comando para listar as collections disponíveis

// TODO: Refatorar o comando run para rodar arquivos em ts e js

use clap::{Parser, Subcommand};
/**
 * Ex:
 * automa list -> lista todas as collections
 *
 * Output:
 * <collection>
 *    <command>
 *    <command>
 *    <command>
 *    <command>
 *
 * <collection>
 *    <command>
 */

use url::Url;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Run {
            command_collection,
            command_name,
        }) => {
            // Check for an existing path on the commands folder with the given collection and name
            let mut command_file_path = PathBuf::new();

            command_file_path.push("commands");
            command_file_path.push(command_collection);
            command_file_path.push(command_name);

            if !command_file_path.exists() {
                return Err("Command not found".into());
            }

            println!(
                "Running command: {} from collection: {}",
                command_name, command_collection
            );


        }

        Some(Commands::Add {
            command_collection,
            command_name,
            command_url,
        }) => {
            if !is_valid_url(command_url) {
                return Err("Invalid URL".into());
            }

            let response = reqwest::get(command_url).await?;

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
