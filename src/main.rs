use notify::{event::CreateKind, EventKind};
use std::sync::mpsc::channel;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use fbr_service::{config, filter_events, Error, FileWatcher};

#[tokio::main]
async fn main() -> Result<(), Error> {
    registry()
        .with(EnvFilter::try_from_default_env().map_or("info".into(), |env| env))
        .with(fmt::layer())
        .init();

    let config = config().await;
    let listen_path = config.listen_path().to_owned();
    let globset = config.globset().to_owned();

    let (tx, rx) = channel();

    let watch =
        tokio::spawn(async move { FileWatcher::new(listen_path).await?.debouncer(tx).await });

    let handle = tokio::spawn(async move {
        loop {
            match rx.recv() {
                Ok(Ok(debounced_events)) => {
                    let events = filter_events(
                        debounced_events,
                        vec![EventKind::Create(CreateKind::File)],
                        globset.clone(),
                    );

                    info!("events: {:?}", events);
                }
                Ok(Err(errors)) => {
                    error!("notify error: {:?}", errors);
                }
                Err(error) => {
                    error!("mpsc recv error: {:?}", error);
                }
            }
        }
    });

    let _ = tokio::join!(watch, handle);

    Ok(())
}
