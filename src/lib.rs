mod config;
mod error;
mod file_watcher;

pub use config::{config, Config};
pub use error::Error;
pub use file_watcher::{filter_events, FileWatcher};
