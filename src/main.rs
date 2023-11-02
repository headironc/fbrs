use globset::GlobSet;
use notify::{event::CreateKind, EventKind};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc::channel, Arc},
};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use fbr_service::{
    config::Config,
    error::Error::{self, MpscRecv, Notifies},
    file_watcher::{filter_events, FileWatcher},
    processor::{IOBuilder, NetworkIOProcessor, Process, Processors},
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

    let network_io_processor = NetworkIOProcessor::default();

    let mut map: HashMap<_, Box<dyn Process>> = HashMap::new();

    map.insert("com.proxy.network.io", Box::new(network_io_processor));

    let processors = Arc::new(Processors::new(map));

    listen(listen_path, globset, Arc::clone(&processors)).await?;

    Ok(())
}

async fn listen(
    listen_path: PathBuf,
    globset: GlobSet,
    processors: Arc<Processors>,
) -> Result<(), Error> {
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

            #[cfg(target_os = "windows")]
            let events = filter_events(
                debounced_events,
                vec![
                    // * CreateKind::Any for windows
                    EventKind::Create(CreateKind::Any),
                ],
                globset.clone(),
            );

            #[cfg(any(target_os = "linux", target_os = "macos"))]
            let events = filter_events(
                debounced_events,
                vec![EventKind::Create(CreateKind::File)],
                globset.clone(),
            );

            let paths = events
                .into_iter()
                .flat_map(|event| event.event.paths)
                .collect::<Vec<_>>();

            if !paths.is_empty() {
                for path in paths {
                    info!("path: {:?}", path);

                    let mut file = File::open(path).await?;

                    let mut buffer = Vec::new();

                    file.read_to_end(&mut buffer).await?;

                    let io_builder = match IOBuilder::new(&buffer) {
                        Ok(io_builder) => io_builder,
                        Err(e) => {
                            error!("parse json error: {}", e);

                            continue;
                        }
                    };

                    let io = io_builder.build()?;

                    processors.process(io).await?;
                }
            }
        }
    })
    .await??;

    Ok(())
}
