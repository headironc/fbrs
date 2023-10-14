use globset::GlobSet;
use notify::{EventKind, FsEventWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use std::{path::PathBuf, sync::mpsc::Sender, time::Duration};
use tokio::fs::create_dir_all;
use tracing::error;

use crate::Error::{self, DirDoesNotExist, NotDirectory};

pub struct FileWatcher {
    path: PathBuf,
}

impl FileWatcher {
    pub async fn new(path: PathBuf) -> Result<Self, Error> {
        if !path.exists() {
            if let Err(error) = create_dir_all(&path).await {
                error!("Failed to create path: {}", error);

                return Err(DirDoesNotExist(error));
            }
        }

        if path.is_file() {
            error!("The specified path is a file");

            return Err(NotDirectory(format!(
                "The specified path is a file: {:?}",
                path
            )));
        }

        Ok(Self { path })
    }

    pub async fn debouncer(
        &self,
        sender: Sender<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    ) -> Result<Debouncer<FsEventWatcher, FileIdMap>, Error> {
        let mut debouncer = new_debouncer(Duration::from_millis(1), None, sender)?;

        debouncer
            .watcher()
            .watch(&self.path, RecursiveMode::NonRecursive)?;

        Ok(debouncer)
    }
}

#[derive(Debug, Clone)]
pub struct FileEvents {
    inner: Vec<DebouncedEvent>,
}

impl FileEvents {
    pub fn new(events: Vec<DebouncedEvent>, kinds: Vec<EventKind>, globset: GlobSet) -> Self {
        let mut events = events;

        events.retain(|event| {
            kinds.contains(&event.kind) && event.paths.iter().any(|path| globset.is_match(path))
        });

        Self { inner: events }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
