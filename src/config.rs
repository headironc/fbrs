use clap::Parser;
use dotenv::{from_filename, var};
use globset::{Glob, GlobSet, GlobSetBuilder};
use once_cell::sync::Lazy;
use std::{path::PathBuf, process::exit};
use tokio::{fs::create_dir_all, sync::OnceCell};
use tracing::{error, info};

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

impl Args {
    /// Check if the config file path is valid
    fn validate(&self) -> bool {
        self.config.exists() && self.config.is_file()
    }
}

static CONFIG: Lazy<OnceCell<Config>> = Lazy::new(OnceCell::new);

impl Config {
    /// Get the config instance
    pub async fn instance() -> &'static Self {
        CONFIG
            .get_or_init(|| {
                let args = Args::parse();

                // check if the specified path exists
                if !args.validate() {
                    error!("The specified config file does not exist");

                    exit(1);
                }

                Self::new(args.config)
            })
            .await
    }

    async fn new(config: PathBuf) -> Self {
        info!("Reading config file...");

        // load the config file
        from_filename(&config).ok();

        let listen_path = Self::get_path_from_env("LISTEN_PATH").await;

        let processor_dir_path = Self::get_path_from_env("PROCESSOR_DIR_PATH").await;

        let globset = Self::build_globset();

        Self {
            listen_path,
            processor_dir_path,
            globset,
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

    /// Get the PathBuff with the given name from the environment
    async fn get_path_from_env(name: &str) -> PathBuf {
        let path_string = match var(name) {
            Ok(path_string) => path_string,
            Err(_) => {
                error!("Did not find {} in config file", name);

                exit(1);
            }
        };

        let path = PathBuf::from(path_string);

        if !path.exists() {
            info!(
                "The path {} does not exist, creating it...",
                path.to_string_lossy()
            );

            if let Err(error) = create_dir_all(&path).await {
                error!(
                    "Failed to create path {}: {}",
                    path.to_string_lossy(),
                    error
                );

                exit(1);
            }

            info!("Successfully created path {}", path.to_string_lossy());
        }

        if !path.is_dir() {
            error!("The path {} is not a directory", path.to_string_lossy());

            exit(1);
        }

        path
    }

    /// Construct a new globset from the given whitelist
    fn build_globset() -> GlobSet {
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

        match builder.build() {
            Ok(set) => set,
            Err(_) => {
                error!("Failed to build whitelist glob set");

                exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use super::*;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use std::env::current_dir;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use tokio::fs::write;

    // Only used in linux os or mac os
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn create_test_config_file() {
        let current_dir = current_dir().unwrap();

        let listen_path = current_dir.join("test/listen");
        let processor_dir_path = current_dir.join("test/processor");

        let config = format!(
            r#"
            LISTEN_PATH="{}"
            PROCESSOR_DIR_PATH="{}"
            WHITELIST="*.txt"
        "#,
            listen_path.to_string_lossy(),
            processor_dir_path.to_string_lossy()
        );

        let config_path = current_dir.join("test_config.env");

        write(&config_path, config).await.unwrap();
    }

    #[tokio::test]
    #[cfg(any(target_os = "linux", target_os = "macos",))]
    async fn test_config() {
        create_test_config_file().await;
        let current_dir = current_dir().unwrap();
        let config_path = current_dir.join("test_config.env");

        let config = Config::new(config_path).await;

        assert_eq!(
            config.listen_path().to_string_lossy(),
            current_dir.join("test/listen").to_string_lossy()
        );

        assert_eq!(
            config.processor_dir_path().to_string_lossy(),
            current_dir.join("test/processor").to_string_lossy()
        );

        assert_eq!(config.globset().len(), 1);
    }
}
