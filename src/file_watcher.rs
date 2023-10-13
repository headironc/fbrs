use globset::GlobSet;
use notify::{EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent};
use std::{future::Future, path::PathBuf, process::exit, sync::mpsc::channel, time::Duration};
use tokio::fs::create_dir_all;
use tracing::error;

use crate::Error;

pub struct FileWatcher {
    path: PathBuf,
    kinds: Vec<EventKind>,
    globset: GlobSet,
}

impl FileWatcher {
    pub async fn new(path: PathBuf, kinds: Vec<EventKind>, globset: GlobSet) -> Self {
        if !path.exists() {
            if let Err(error) = create_dir_all(&path).await {
                error!("Failed to create path: {}", error);

                exit(1);
            }
        }

        if path.is_file() {
            error!("The specified path is a file");

            exit(1);
        }

        Self {
            path,
            kinds,
            globset,
        }
    }

    pub async fn debouncer<F, Fut>(&self, f: F) -> Result<(), Error>
    where
        F: Fn(FileEvents) -> Fut,
        Fut: Future<Output = Result<(), Error>>,
    {
        let (tx, rx) = channel();

        let mut debouncer = new_debouncer(Duration::from_millis(1), None, tx)?;

        debouncer
            .watcher()
            .watch(&self.path, RecursiveMode::NonRecursive)?;

        loop {
            match rx.recv()? {
                Ok(events) => {
                    let mut events = FileEvents::new(events);
                    events.filter(&self.kinds, &self.globset);

                    if !events.is_empty() {
                        f(events).await?;
                    }
                }
                Err(e) => {
                    error!("error: {:?}", e);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEvents {
    inner: Vec<DebouncedEvent>,
}

impl FileEvents {
    pub fn new(events: Vec<DebouncedEvent>) -> Self {
        Self { inner: events }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// filter out events that are not of the specified type
    pub fn filter(&mut self, kinds: &[EventKind], globset: &GlobSet) {
        self.inner.retain(|event| {
            kinds.contains(&event.kind) && event.paths.iter().any(|path| globset.is_match(path))
        });
    }
}
