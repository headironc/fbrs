use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;
use std::{error::Error, path::Path, sync::mpsc::channel, time::Duration};
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use fbr_service::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    registry()
        .with(EnvFilter::try_from_default_env().map_or("info".into(), |env| env))
        .with(fmt::layer())
        .init();

    let _config = Config::new().await;

    Ok(())
}

pub fn test() -> Result<(), Box<dyn Error>> {
    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(1), None, tx)?;

    debouncer.watcher().watch(
        Path::new("/Users/headiron/Desktop"),
        RecursiveMode::NonRecursive,
    )?;

    loop {
        match rx.recv()? {
            Ok(mut events) => {
                // 过滤
                events.retain(|event| {
                    event.paths.iter().any(|path| {
                        path.file_name()
                            .map_or_else(|| true, |name| name != ".DS_Store")
                    })
                });

                println!("{:#?}", events)
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
