//! `toile-harness` — a headless, deterministic visual test harness for the
//! Toile engine. It boots a real GPU device with no window, renders with the
//! engine's actual renderers into an off-screen target, and reads the result
//! back as pixels / PNG so changes can be verified the way Playwright verifies
//! a web page: by looking at what actually rendered.
//!
//! See [`Headless`] for the raw GPU target and [`Harness`] for scene snapshots.

pub mod headless;
pub mod snapshot;

pub use headless::Headless;
pub use snapshot::{Harness, SnapshotOptions};
