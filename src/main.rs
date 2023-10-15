use notify::{event::CreateKind, EventKind};
use std::sync::mpsc::channel;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use fbr_service::{
    config, filter_events,
    Error::{self, MpscRecv, Notifies},
    FileWatcher,
};

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
                vec![EventKind::Create(CreateKind::File)],
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
