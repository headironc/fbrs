use globset::GlobSet;
use notify::{event::CreateKind, EventKind};
use std::{path::PathBuf, sync::mpsc::channel};
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use fbr_service::{
    filter_events, Config,
    Error::{self, MpscRecv, Notifies},
    FileWatcher,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    registry()
        .with(EnvFilter::try_from_default_env().map_or("info".into(), |env| env))
        .with(fmt::layer())
        .init();

    let config = Config::instance().await;
    let listen_path = config.listen_path().to_owned();
    let globset = config.globset().to_owned();

    listen(listen_path, globset).await?;

    Ok(())
}

async fn listen(listen_path: PathBuf, globset: GlobSet) -> Result<(), Error> {
    let (tx, rx) = channel();

    // `let _debouncer`, avoid dropping the debouncer immediately, which will cause dropping the tx, and then the rx will be closed.
    let _debouncer =
        tokio::spawn(async move { FileWatcher::new(listen_path).await?.debouncer(tx) }).await??;

    tokio::spawn(async move {
        loop {
            let res = match rx.recv() {
                Ok(res) => res,
                Err(e) => {
                    error!("mpsc recv error: {}", e);

                    break Err::<(), Error>(MpscRecv(e));
                }
            };

            let debounced_events = match res {
                Ok(debounced_events) => debounced_events,
                Err(errors) => {
                    error!("notify errors: {:?}", errors);

                    break Err(Notifies(errors));
                }
            };

            let events = filter_events(
                debounced_events,
                vec![
                    EventKind::Create(CreateKind::File),
                    // * CreateKind::Any for windows
                    EventKind::Create(CreateKind::Any),
                ],
                globset.clone(),
            );

            if !events.is_empty() {
                info!("events: {:?}", events);
            }
        }
    })
    .await??;

    Ok(())
}
