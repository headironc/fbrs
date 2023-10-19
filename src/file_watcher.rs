use globset::GlobSet;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
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

    pub fn debouncer(
        &self,
        sender: Sender<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    ) -> Result<Debouncer<RecommendedWatcher, FileIdMap>, Error> {
        let mut debouncer = new_debouncer(Duration::from_millis(1), None, sender)?;

        debouncer
            .watcher()
            .watch(&self.path, RecursiveMode::NonRecursive)?;

        Ok(debouncer)
    }
}

pub fn filter_events(
    events: Vec<DebouncedEvent>,
    kinds: Vec<EventKind>,
    globset: GlobSet,
) -> Vec<DebouncedEvent> {
    let mut events = events;

    events.retain(|event| {
        kinds.contains(&event.kind) && event.paths.iter().any(|path| globset.is_match(path))
    });

    events
}

#[cfg(test)]
mod tests {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use super::*;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use globset::{Glob, GlobSetBuilder};
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use notify::{event::CreateKind, Event};
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    use std::{sync::mpsc::channel, time::Instant};

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn test_filter_events() {
        fn create_event(kind: EventKind, path: &str) -> DebouncedEvent {
            DebouncedEvent::new(Event::new(kind).add_path(path.into()), Instant::now())
        }

        let events = vec![
            create_event(EventKind::Create(CreateKind::File), "foo.txt"),
            create_event(EventKind::Create(CreateKind::Folder), "text.txt"),
            create_event(EventKind::Create(CreateKind::File), "bar.json"),
        ];

        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new("*.txt").unwrap());
        let globset = builder.build().unwrap();

        let events = filter_events(events, vec![EventKind::Create(CreateKind::File)], globset);

        assert_eq!(events.len(), 1);
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn test_filter_events_no_match() {
        fn create_event(kind: EventKind, path: &str) -> DebouncedEvent {
            DebouncedEvent::new(Event::new(kind).add_path(path.into()), Instant::now())
        }

        let events = vec![
            create_event(EventKind::Create(CreateKind::File), "foo.txt"),
            create_event(EventKind::Create(CreateKind::Folder), "text.txt"),
            create_event(EventKind::Create(CreateKind::Other), "bar.json"),
        ];

        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new("*.json").unwrap());
        let globset = builder.build().unwrap();

        let events = filter_events(events, vec![EventKind::Create(CreateKind::File)], globset);

        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn test_file_watcher() {
        let path = PathBuf::from("./test_file_watcher");

        std::fs::create_dir_all(&path).unwrap();

        let file_watcher = FileWatcher::new(path.clone()).await.unwrap();

        let (tx, rx) = channel();

        let _debouncer = file_watcher.debouncer(tx).unwrap();

        std::fs::write(path.join("foo.txt"), "foo").unwrap();

        match rx.recv() {
            Ok(Ok(debounced_events)) => {
                println!("events: {:?}", debounced_events);

                let events = filter_events(
                    debounced_events,
                    vec![EventKind::Create(CreateKind::File)],
                    GlobSetBuilder::new()
                        .add(Glob::new("*.txt").unwrap())
                        .build()
                        .unwrap(),
                );

                assert_eq!(events.len(), 1);
            }
            Ok(Err(errors)) => {
                error!("notify error: {:?}", errors);
            }
            Err(error) => {
                error!("mpsc recv error: {:?}", error);
            }
        }
    }
}
