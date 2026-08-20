use std::path::Path;
use winit::window::Icon;

/// Loads a window icon from an image file.
///
/// # Requirements
/// - Format: PNG (recommended)
/// - Size: multiple of 16 (16x16, 32x32, 64x64, 128x128, 256x256)
/// - Channels: RGBA (with alpha)
///
/// # Example
/// ```rust,ignore
/// use runa_asset::load_window_icon;
///
/// let icon = load_window_icon("assets/icon.png")?;
/// # let _ = icon;
/// # Ok::<(), String>(())
/// ```
pub fn load_window_icon<P: AsRef<Path>>(path: P) -> Result<Icon, String> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)
        .map_err(|e| format!("Failed to read icon '{}': {}", path.display(), e))?;
    load_window_icon_from_bytes(&bytes)
}

pub fn load_window_icon_from_bytes(bytes: &[u8]) -> Result<Icon, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Failed to decode icon image: {}", e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width != height {
        return Err(format!("Icon must be square, got {}x{}", width, height));
    }

    const VALID_SIZES: &[u32] = &[16, 32, 48, 64, 128, 256, 512];
    if !VALID_SIZES.contains(&width) {
        return Err(format!(
            "Icon size {}x{} is not standard. Recommended: 16, 32, 64, 128, 256, 512",
            width, height
        ));
    }

    Icon::from_rgba(rgba.to_vec(), width, height)
        .map_err(|e| format!("Failed to create icon: {}", e))
}

/// Loads multiple icon sizes (recommended for cross-platform support).
///
/// # Example
/// ```rust,ignore
/// use runa_asset::load_window_icons;
///
/// let icons = load_window_icons(&[
///     "assets/icon_16.png",
///     "assets/icon_32.png",
///     "assets/icon_64.png",
///     "assets/icon_256.png",
/// ])?;
/// # let _ = icons.first().cloned();
/// # Ok::<(), String>(())
/// ```
#[allow(dead_code)]
pub fn load_window_icons<P: AsRef<Path>>(paths: &[P]) -> Result<Vec<Icon>, String> {
    let mut icons = Vec::new();

    for path in paths {
        match load_window_icon(path) {
            Ok(icon) => icons.push(icon),
            Err(e) => eprintln!("⚠️  Skipping icon '{}': {}", path.as_ref().display(), e),
        }
    }

    if icons.is_empty() {
        return Err("No valid icons loaded".to_string());
    }

    Ok(icons)
}
