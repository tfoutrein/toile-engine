use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use log;

/// Identifies an async-loading asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsyncAssetId(pub u64);

/// Type of asset to load.
#[derive(Debug, Clone)]
pub enum AssetKind {
    Texture,
    Sound,
    Json,
}

/// Raw decoded asset data (ready for GPU upload on main thread).
pub enum RawAsset {
    Texture {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
    Sound {
        bytes: Vec<u8>,
    },
    Json {
        text: String,
    },
}

struct LoadRequest {
    id: AsyncAssetId,
    path: PathBuf,
    kind: AssetKind,
    simulated_delay_ms: u64,
}

/// Status of an asset being loaded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssetStatus {
    Loading,
    Ready,
    Failed,
}

/// Completed asset from the background thread.
pub struct CompletedAsset {
    pub id: AsyncAssetId,
    pub path: PathBuf,
    pub result: Result<RawAsset, String>,
}

/// Background asset loader. Loads and decodes files on a worker thread,
/// sends raw data back to the main thread for GPU upload.
pub struct AsyncLoader {
    tx: mpsc::Sender<LoadRequest>,
    rx: mpsc::Receiver<CompletedAsset>,
    statuses: std::collections::HashMap<AsyncAssetId, AssetStatus>,
    next_id: u64,
    total_requested: u32,
    total_completed: u32,
}

impl AsyncLoader {
    pub fn new() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<LoadRequest>();
        let (done_tx, done_rx) = mpsc::channel::<CompletedAsset>();

        // Spawn worker thread
        thread::spawn(move || {
            while let Ok(req) = req_rx.recv() {
                if req.simulated_delay_ms > 0 {
                    thread::sleep(std::time::Duration::from_millis(req.simulated_delay_ms));
                }
                let result = Self::load_file(&req.path, &req.kind);
                let _ = done_tx.send(CompletedAsset {
                    id: req.id,
                    path: req.path,
                    result,
                });
            }
        });

        Self {
            tx: req_tx,
            rx: done_rx,
            statuses: std::collections::HashMap::new(),
            next_id: 1,
            total_requested: 0,
            total_completed: 0,
        }
    }

    /// Queue an asset for background loading.
    pub fn request(&mut self, path: &Path, kind: AssetKind) -> AsyncAssetId {
        self.request_with_delay(path, kind, 0)
    }

    /// Queue an asset with a simulated delay (for demo/testing).
    pub fn request_with_delay(&mut self, path: &Path, kind: AssetKind, delay_ms: u64) -> AsyncAssetId {
        let id = AsyncAssetId(self.next_id);
        self.next_id += 1;
        self.total_requested += 1;

        self.statuses.insert(id, AssetStatus::Loading);
        let _ = self.tx.send(LoadRequest {
            id,
            path: path.to_path_buf(),
            kind,
            simulated_delay_ms: delay_ms,
        });

        id
    }

    /// Poll for completed assets (non-blocking). Returns newly completed assets.
    pub fn poll(&mut self) -> Vec<CompletedAsset> {
        let mut completed = Vec::new();
        while let Ok(asset) = self.rx.try_recv() {
            let status = if asset.result.is_ok() {
                AssetStatus::Ready
            } else {
                AssetStatus::Failed
            };
            self.statuses.insert(asset.id, status);
            self.total_completed += 1;
            completed.push(asset);
        }
        completed
    }

    /// Check if a specific asset is loaded.
    pub fn status(&self, id: AsyncAssetId) -> AssetStatus {
        self.statuses
            .get(&id)
            .copied()
            .unwrap_or(AssetStatus::Loading)
    }

    /// Check if all requested assets are done (ready or failed).
    pub fn all_done(&self) -> bool {
        self.total_completed >= self.total_requested
    }

    /// Loading progress (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        if self.total_requested == 0 {
            return 1.0;
        }
        self.total_completed as f32 / self.total_requested as f32
    }

    /// Reset counters (call after a loading phase is complete).
    pub fn reset_progress(&mut self) {
        self.total_requested = 0;
        self.total_completed = 0;
    }

    fn load_file(path: &Path, kind: &AssetKind) -> Result<RawAsset, String> {
        match kind {
            AssetKind::Texture => {
                let img = image::open(path)
                    .map_err(|e| format!("Failed to load image {}: {e}", path.display()))?
                    .into_rgba8();
                let (w, h) = img.dimensions();
                Ok(RawAsset::Texture {
                    rgba: img.into_raw(),
                    width: w,
                    height: h,
                })
            }
            AssetKind::Sound => {
                let bytes = std::fs::read(path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                Ok(RawAsset::Sound { bytes })
            }
            AssetKind::Json => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                Ok(RawAsset::Json { text })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_nonexistent_file() {
        let mut loader = AsyncLoader::new();
        let id = loader.request(Path::new("nonexistent.png"), AssetKind::Texture);
        assert_eq!(loader.status(id), AssetStatus::Loading);

        // Wait for worker
        std::thread::sleep(std::time::Duration::from_millis(100));
        let completed = loader.poll();
        assert_eq!(completed.len(), 1);
        assert!(completed[0].result.is_err());
        assert_eq!(loader.status(id), AssetStatus::Failed);
    }

    #[test]
    fn progress_tracking() {
        let mut loader = AsyncLoader::new();
        assert_eq!(loader.progress(), 1.0); // nothing to load

        loader.request(Path::new("a.png"), AssetKind::Texture);
        loader.request(Path::new("b.png"), AssetKind::Texture);
        assert_eq!(loader.progress(), 0.0);

        std::thread::sleep(std::time::Duration::from_millis(100));
        loader.poll();
        assert!(loader.all_done());
        assert_eq!(loader.progress(), 1.0);
    }
}
