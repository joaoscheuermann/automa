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
use either::Either;
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
use octocrab;
use reqwest::header::ACCEPT;

use url::Url;

use std::fs::File;
use std::io::prelude::*;
use std::io::Bytes;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;
use zip::ZipArchive;

static DENO_DIR_NAME: &str = "deno";
static COMMANDS_DIR_NAME: &str = "commands";

static GITHUB_APP_CLIENT_ID: &str = "Iv1.60a4682a0d3c1ca2";

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

    /// Authenticates the user with GitHub.
    /// This is required to be able to access private gists and do a backup of the commands.
    Auth {},
}

fn get_root_dir() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();

    path.pop();

    path
}

fn get_deno_dir(root: &PathBuf) -> PathBuf {
    let root_srt = root.to_str().expect("Failed to get root dir");

    PathBuf::from_iter([root_srt, DENO_DIR_NAME].iter())
}

fn get_commands_dir(root: &PathBuf) -> PathBuf {
    let root_srt = root.to_str().expect("Failed to get root dir");

    PathBuf::from_iter([root_srt, COMMANDS_DIR_NAME].iter())
}

fn get_collection_dir(root: &PathBuf, command_collection: &str) -> PathBuf {
    let mut path = get_commands_dir(root);

    path.push(command_collection);

    path
}

fn get_command_dir(root: &PathBuf, command_collection: &str, command_name: &str) -> PathBuf {
    let mut path = get_collection_dir(root, command_collection);

    path.push(command_name);

    path
}

fn is_valid_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn get_deno_asset_release_name() -> Result<&'static str, &'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match os {
        "windows" => match arch {
            "x86_64" => Ok("deno-x86_64-pc-windows-msvc.zip"),

            _ => Err("Unsupported architecture"),
        },

        "linux" => match arch {
            "x86_64" => Ok("deno-x86_64-unknown-linux-gnu.zip"),

            _ => Err("Unsupported architecture"),
        },

        "macos" => match arch {
            "aarch64" => Ok("deno-aarch64-apple-darwin.zip"),

            "x86_64" => Ok("deno-x86_64-apple-darwin.zip"),

            _ => Err("Unsupported architecture"),
        },

        _ => Err("Unsupported OS"),
    }
}

fn get_deno_executable_name() -> Result<&'static str, &'static str> {
    let os = std::env::consts::OS;

    match os {
        "windows" => Ok("deno.exe"),

        "linux" => Ok("deno"),

        "macos" => Ok("deno"),

        _ => Err("Unsupported OS"),
    }
}

fn ensure_path_is_created(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        std::fs::create_dir_all(path.clone())?;
    }

    Ok(())
}

fn save_file(path: &PathBuf, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(path).expect("Failed to create file");

    file.write_all(bytes)?;

    Ok(())
}

fn unzip_bytes(path: &PathBuf, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;

    for i in 0..archive.len() {
        let file = archive.by_index(i).expect("Failed to get file");

        let file_name = file.name();
        let mut file_path = path.clone();

        file_path.push(file_name);

        print!("Saving file: {}...", file_name);
        println!("{:?}", file_path);

        let bytes = file
            .bytes()
            .collect::<Result<Vec<u8>, _>>()
            .expect("Failed to get bytes");

        let slice = bytes.as_slice();

        save_file(&file_path, slice)?;
    }

    Ok(())
}

async fn download_deno_latest_release(
    deno_system_release_name: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let latest_release = octocrab::instance()
        .repos("denoland", "deno")
        .releases()
        .get_latest()
        .await
        .expect("Failed to get latest release");

    let asset = latest_release
        .assets
        .iter()
        .find(|asset| asset.name == deno_system_release_name)
        .expect("Failed to find asset");

    let result = reqwest::get(asset.browser_download_url.as_str())
        .await
        .expect("Failed to download asset");

    let bytes = result.bytes().await?;

    Ok(bytes.to_vec())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let root_path = get_root_dir();
    let deno_dir = get_deno_dir(&root_path);
    let commands_dir = get_commands_dir(&root_path);

    let deno_system_release_name = get_deno_asset_release_name()?;
    let deno_executable_name = get_deno_executable_name()?;
    let deno_executable_path = deno_dir.join(deno_executable_name);

    // Installs deno if it's not installed
    if !deno_executable_path.exists() {
        println!("Deno is not installed. Installing...");

        let bytes = download_deno_latest_release(deno_system_release_name)
            .await
            .expect("Failed to download deno release");

        ensure_path_is_created(&deno_executable_path).expect("Failed to create path");

        unzip_bytes(&deno_executable_path, bytes.as_slice()).expect("Failed to unzip bytes");
    }

    match &cli.command {
        Some(Commands::Run {
            command_collection,
            command_name,
        }) => {
            // Check for an existing path on the commands folder with the given collection and name
            // let mut command_file_path = PathBuf::new();

            // command_file_path.push("commands");
            // command_file_path.push(command_collection);
            // command_file_path.push(command_name);

            // if !command_file_path.exists() {
            //     return Err("Command not found".into());
            // }

            // println!(
            //     "Running command: {} from collection: {}",
            //     command_name, command_collection
            // );
        }

        // octocrab::instance().authenticate_as_device(, scope).await?;
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

        Some(Commands::Auth {}) => {
            let client_id = secrecy::Secret::from(GITHUB_APP_CLIENT_ID.to_string());
            let scope = ["gist"];

            let crab = octocrab::Octocrab::builder()
                .base_uri("https://github.com")?
                .add_header(ACCEPT, "application/json".to_string())
                .build()?;

            let codes = crab.authenticate_as_device(&client_id, scope).await?;

            println!(
                "Go to {} and enter code {}",
                codes.verification_uri, codes.user_code
            );

            let mut interval = Duration::from_secs(codes.interval);
            let mut clock = tokio::time::interval(interval);

            loop {
                clock.tick().await;
                match codes.poll_once(&crab, &client_id).await? {
                    Either::Left(_) => break (),
                    Either::Right(cont) => {
                        match cont {
                            octocrab::auth::Continue::SlowDown => {
                                // We were request to slow down. We add five seconds to the polling
                                // duration.
                                interval += Duration::from_secs(5);
                                clock = tokio::time::interval(interval);
                                // The first tick happens instantly, so we tick that off immediately.
                                clock.tick().await;
                            }
                            octocrab::auth::Continue::AuthorizationPending => {
                                // The user has not clicked authorize yet, but nothing has gone wrong.
                                // We keep polling.
                            }
                        }
                    }
                }
            }

            println!("Authorization succeeded!");
        }

        None => {
            println!("Missing command!");
        }
    }

    Ok(())
}
