use std::path::Path;

use crate::model::EventSheet;

#[derive(Debug, thiserror::Error)]
pub enum EventSheetError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn load_event_sheet(path: &Path) -> Result<EventSheet, EventSheetError> {
    let json = std::fs::read_to_string(path)?;
    let sheet: EventSheet = serde_json::from_str(&json)?;
    Ok(sheet)
}

pub fn save_event_sheet(path: &Path, sheet: &EventSheet) -> Result<(), EventSheetError> {
    let json = serde_json::to_string_pretty(sheet)?;
    std::fs::write(path, json)?;
    Ok(())
}
