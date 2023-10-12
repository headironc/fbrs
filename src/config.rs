use clap::Parser;
use dotenv::{from_filename, var};
use std::{
    path::{Path, PathBuf},
    process::exit,
};
use tokio::fs::create_dir_all;
use tracing::{error, info};

pub struct Config {
    listen_path: PathBuf,
    processor_dir_path: PathBuf,
}

#[derive(Debug, Parser)]
#[command(author, version)]
struct Args {
    /// Set a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

impl Config {
    pub async fn new() -> Self {
        let args = Args::parse();

        // check if the specified path exists
        if !args.config.exists() {
            error!("The specified path does not exist");

            exit(1);
        }

        // check if there is a file at the specified path
        if !Path::is_file(&args.config) {
            error!("There is no config file at the specified path");

            exit(1);
        }

        info!("Reading config file...");

        // load the config file
        from_filename(&args.config).ok();

        // get the listen path from the environment
        let listen_path = get_path_from_env("LISTEN_PATH").await;

        // get the processor dir path from the environment
        let processor_dir_path = get_path_from_env("PROCESSOR_DIR_PATH").await;

        info!("Read config file successfully");

        Self {
            listen_path,
            processor_dir_path,
        }
    }

    pub fn listen_path(&self) -> &PathBuf {
        &self.listen_path
    }

    pub fn processor_dir_path(&self) -> &PathBuf {
        &self.processor_dir_path
    }
}

/// Get a path from the environment
async fn get_path_from_env(name: &str) -> PathBuf {
    match var(name) {
        Ok(path) => {
            if path.is_empty() {
                error!("{} has no value", name);

                exit(1);
            }

            match path.parse::<PathBuf>() {
                Ok(path) => {
                    if !path.exists() {
                        info!("The specified {} does not exist, creating...", name);

                        match create_dir_all(&path).await {
                            Ok(_) => info!("Successfully created {}: {}", name, path.display()),
                            Err(e) => {
                                error!("Failed to create {}: {}", name, e);

                                exit(1);
                            }
                        }
                    }

                    path
                }
                Err(_) => {
                    error!("Failed to parse {} from config file", name);

                    exit(1);
                }
            }
        }
        Err(_) => {
            error!("Did not find {} in config file", name);

            exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::set_var;

    #[tokio::test]
    async fn test_get_path_from_env() {
        set_var("LISTEN_PATH", "/tmp/listen_path");

        let path = get_path_from_env("LISTEN_PATH").await;

        assert_eq!(path, PathBuf::from("/tmp/listen_path"));
    }
}
