//! Regenerates `crates/runa_script_api/scripts/runa_base.luau` — the engine's
//! committed built-in Luau type layer.
//!
//! The binary links the whole engine (`runa_engine` → `runa_core` →
//! `runa_render_api`), so `inventory` picks up every `#[script(builtin)]` type and
//! the aux types registered by the engine crates. Run it whenever a built-in
//! component / aux type changes:
//!
//! ```text
//! cargo run -p runa_engine --bin gen_runa_base
//! ```
//!
//! The generated file is committed so user projects only need `include_str!`
//! (via `runa_script_api`) and never the engine on disk.

use std::path::PathBuf;

use runa_engine::scripting_api;

fn main() {
    let out =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../runa_script_api/scripts/runa_base.luau");
    let base = out.parent().unwrap_or(&out);
    if !base.exists() {
        std::fs::create_dir_all(base).expect("create scripts/ dir");
    }
    scripting_api::write_runa_base(&out);
    eprintln!("wrote {}", out.display());
}
