# Contributing to Runa Engine

Thanks for your interest! Runa is a community-driven project, and every
contribution — whether a bug report, a doc fix, a performance improvement,
or a new feature — is very welcome.

## Quick Start

1. Fork the repo on GitHub.
2. Clone your fork:
   ```bash
   git clone https://github.com/RunaLight/runa.git
   cd runa
   ```
3. Build the project:
   ```bash
   cargo build
   ```
4. Run the sandbox example to verify everything works:
   ```bash
   cargo run -p sandbox
   ```

## Project Structure

```
crates/
  runa_core/        # Object-Component-Script system, input, audio, rendering API
  runa_render/      # wgpu renderer (2D sprites, 3D meshes, UI, background)
  runa_app/         # Window bootstrap and app loop (winit)
  runa_asset/       # Asset loading (textures, audio, fonts)
  runa_project/     # Project manifests, world serialization, scaffolding
  runa_editor/      # Optional egui-based editor
  runa_engine/      # Umbrella crate for game code
  runa_macros/      # Derive macros
  runa_render_api/  # Render command queue (no GPU dependency)
examples/
  sandbox/          # Main 2D sandbox
  sandbox_3d/       # 3D example
  sandbox_ui/       # UI example
  sandbox_soundtest/# Audio example
docs/
  tutorials/        # Step-by-step guides
  architecture/     # Architecture decision notes
```

## Code Philosophy

- **Code-first**: Rust code is the primary way to build a game. The editor
  is a future concern — we stabilise the code API first.
- **Zero-cost**: unused features must not affect runtime performance. Use
  Cargo feature flags and `#[cfg]`.
- **Clarity**: explicit is better than magic. Typed archetypes, explicit
  registration, no hidden state.
- **Performance**: O(1) lookups, pooled buffers, no per-frame allocations
  in hot paths. If you add a system, add a benchmark too.

## Pull Request Checklist

Before submitting a PR:

1. **Tests pass**: `cargo test --workspace`
2. **No warnings**: `cargo clippy --workspace -- -D warnings`
3. **Formatted**: `cargo fmt --all`
4. **Updated docs**: if you change public API, update the relevant tutorial
   or architecture doc
5. **Changelog entry**: add a line under `[Unreleased]` in `CHANGELOG.md`

### PR Title Convention

```
type(scope): brief description
```

Types: `feat`, `fix`, `perf`, `docs`, `refactor`, `test`, `ci`
Scope: which crate or area

Examples:

- `feat(runa_core): add EventBus subscribe/unsubscribe`
- `perf(runa_render): pool mesh GPU buffers`
- `docs(tutorials): fix Camera::new_perspective signature`

## Coding Standards

- Follow existing code style (rustfmt default).
- Use the same patterns as surrounding code.
- Public items should have doc comments with examples where practical.
- Feature-gate optional dependencies — never add a required dependency
  that only some users need.
- Avoid `unsafe` unless absolutely necessary and documented with safety
  invariants.

## Where to Start

Check the [ROADMAP.md](ROADMAP.md) for upcoming work. Good first issues
are tagged `good first issue` on GitHub.

Areas that always need help:

- **Documentation**: writing tutorials, doc comments, fixing drift
- **Performance profiling**: finding and fixing hot spots
- **Test coverage**: adding tests for existing features
- **Bug fixes**: fixing issues found by users or CI

## License

By contributing, you agree that your contributions will be dual-licensed
under MIT and Apache 2.0 (same as the project).
