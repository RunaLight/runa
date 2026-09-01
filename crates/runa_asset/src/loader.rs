//! Runa asset loading for images, audio, and more.
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use crate::handle::Handle;
use crate::texture::TextureAsset;

pub use crate::audio::{AudioAsset, AudioLoadError};

static IMAGE_CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<TextureAsset>>>> = OnceLock::new();

pub fn load_image(cargo: &str, path: &str) -> Handle<TextureAsset> {
    let full_path = PathBuf::from(cargo).join(path);
    let cache = IMAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let mut guard = cache.lock().unwrap();
    match guard.get(&full_path) {
        Some(arc) => Handle { inner: arc.clone() },
        None => {
            let arc = Arc::new(TextureAsset::load(&full_path).expect("Failed to load image"));
            guard.insert(full_path, arc.clone());
            Handle { inner: arc }
        }
    }
}

/// Load image/texture asset at compile time (with caching)
#[macro_export]
macro_rules! load_image {
    ($path:literal) => {{
        // Compile-time validation
        const _: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path));

        // Runtime loading
        $crate::loader::load_image(env!("CARGO_MANIFEST_DIR"), $path)
    }};
}

/// Load audio asset at compile time (with caching)
#[macro_export]
macro_rules! load_audio {
    ($path:literal) => {{
        use std::sync::Arc;
        use std::sync::OnceLock;

        const _: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path));

        static CACHE: OnceLock<Arc<$crate::AudioAsset>> = OnceLock::new();

        CACHE
            .get_or_init(|| {
                Arc::new($crate::AudioAsset::from_file(env!("CARGO_MANIFEST_DIR"), $path).unwrap())
            })
            .clone()
    }};
}
