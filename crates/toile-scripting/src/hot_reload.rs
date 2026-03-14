use std::path::{Path, PathBuf};
use std::sync::mpsc;

use notify::{Event, EventKind, RecursiveMode, Watcher};

/// Watches a directory for .lua file changes.
pub struct ScriptWatcher {
    _watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<PathBuf>,
}

impl ScriptWatcher {
    pub fn new(scripts_dir: &Path) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    for path in event.paths {
                        if path.extension().is_some_and(|e| e == "lua") {
                            let _ = tx.send(path);
                        }
                    }
                }
            }
        })?;

        watcher.watch(scripts_dir, RecursiveMode::Recursive)?;
        log::info!("Watching for script changes: {}", scripts_dir.display());

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Drain pending file change events (non-blocking).
    pub fn poll_changes(&self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        while let Ok(path) = self.rx.try_recv() {
            if !changed.contains(&path) {
                changed.push(path);
            }
        }
        changed
    }
}
