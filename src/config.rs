use clap::Parser;
use dotenv::{from_filename, var};
use globset::{Glob, GlobSet, GlobSetBuilder};
use once_cell::sync::Lazy;
use std::{path::PathBuf, process::exit};
use tokio::{
    fs::create_dir_all,
    sync::{Mutex, OnceCell},
};
use tracing::{error, info};

static CONFIG: Lazy<Mutex<OnceCell<Config>>> = Lazy::new(|| Mutex::new(OnceCell::new()));

/// Get the initialized config or initialize it from the config file
pub async fn get_or_init_config() -> Config {
    let guard = CONFIG.lock().await;

    match guard.get() {
        Some(config) => config.clone(),
        None => {
            let config = Config::new().await;

            if let Err(e) = guard.set(config.clone()) {
                error!("Failed to set config: {}", e);

                exit(1);
            }

            config
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    listen_path: PathBuf,
    processor_dir_path: PathBuf,
    globset: GlobSet,
}

#[derive(Debug, Parser)]
#[command(author, version)]
struct Args {
    /// Set a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

impl Config {
    async fn new() -> Self {
        let args = Args::parse();

        // check if the specified path exists
        if !args.config.exists() {
            error!("The specified path does not exist");

            exit(1);
        }

        // check if there is a file at the specified path
        if !args.config.is_file() {
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

        let whitelist = match var("WHITELIST") {
            Ok(whitelist) => {
                if whitelist.is_empty() {
                    vec![]
                } else {
                    whitelist
                        .split(',')
                        .map(|s| s.into())
                        .collect::<Vec<String>>()
                }
            }
            Err(_) => {
                info!("Did not find WHITELIST in config file, using blank whitelist");
                vec![]
            }
        };

        let mut builder = GlobSetBuilder::new();

        for pattern in whitelist {
            let glob = match Glob::new(&pattern) {
                Ok(glob) => glob,
                Err(_) => {
                    error!("Failed to parse whitelist pattern: {}", pattern);

                    exit(1);
                }
            };

            builder.add(glob);

            info!("Added whitelist pattern: {}", pattern);
        }

        let set = match builder.build() {
            Ok(set) => set,
            Err(_) => {
                error!("Failed to build whitelist glob set");

                exit(1);
            }
        };

        Self {
            listen_path,
            processor_dir_path,
            globset: set,
        }
    }

    pub fn listen_path(&self) -> &PathBuf {
        &self.listen_path
    }

    pub fn processor_dir_path(&self) -> &PathBuf {
        &self.processor_dir_path
    }

    pub fn globset(&self) -> &GlobSet {
        &self.globset
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
