//! Thumbnail generation for the asset browser.

use std::path::Path;

const THUMB_SIZE: u32 = 128;

/// Generate a thumbnail for an image file.
/// Saves a 128×128 PNG to the output path.
pub fn generate_thumbnail(source: &Path, output: &Path) -> Result<(), String> {
    let img = image::open(source)
        .map_err(|e| format!("Cannot open image {}: {e}", source.display()))?;

    let thumb = img.thumbnail(THUMB_SIZE, THUMB_SIZE);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create thumb dir: {e}"))?;
    }

    thumb.save(output)
        .map_err(|e| format!("Cannot save thumbnail: {e}"))?;

    Ok(())
}

/// Generate a thumbnail showing the first frame of a spritesheet.
pub fn generate_spritesheet_thumbnail(
    source: &Path,
    output: &Path,
    frame_width: u32,
    frame_height: u32,
) -> Result<(), String> {
    let img = image::open(source)
        .map_err(|e| format!("Cannot open image: {e}"))?;

    // Crop the first frame
    let cropped = img.crop_imm(0, 0, frame_width.min(img.width()), frame_height.min(img.height()));
    let thumb = cropped.thumbnail(THUMB_SIZE, THUMB_SIZE);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create thumb dir: {e}"))?;
    }

    thumb.save(output)
        .map_err(|e| format!("Cannot save thumbnail: {e}"))?;

    Ok(())
}

/// Get image dimensions without loading the full image.
pub fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}
