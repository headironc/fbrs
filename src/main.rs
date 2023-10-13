use notify::{event::CreateKind, EventKind};
use std::error::Error;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

use fbr_service::{get_or_init_config, FileWatcher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    registry()
        .with(EnvFilter::try_from_default_env().map_or("info".into(), |env| env))
        .with(fmt::layer())
        .init();

    let config = get_or_init_config().await;

    FileWatcher::new(
        config.listen_path().clone(),
        vec![EventKind::Create(CreateKind::File)],
        config.globset().clone(),
    )
    .await
    .debouncer(|events| async move {
        info!("events: {:#?}", events);

        Ok(())
    })
    .await?;

    Ok(())
}
