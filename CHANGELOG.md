# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.6] - 2026-08-05

### Bug Fixes

- Camera respects entity Transform (closes [#12](https://github.com/RunaLight/runa/issues/12))
- Changed RunaApp creation methods
- Transform interpolation now used at render time
- _(changelog)_ Preserve header on prepend and linkify issues

### Documentation

- Remove deprecated OCS migration notices from guides
- Remove stale OCS migration banners and tidy up guides

### Styling

- Cargo fmt --all

## [0.7.5] - 2026-07-28

### Changed

- **EventBus** is now a global singleton (`OnceLock<Mutex<...>>`), like `InputState` / `AudioEngine`. No more `world.spawn((EventBus::new(),))` — just `EventBus::emit(event)` and `EventBus::subscribe(cb)` from anywhere.

## [0.7.4] - 2026-07-28

### Added

- **Sorting component** is now respected during rendering. If an entity has a `Sorting` component, its `order` field overrides the draw order for both sprites (`SpriteRenderer`) and 3D meshes (`MeshRenderer`). Entities without `Sorting` keep `order: 0`.

## [0.7.3] - 2026-07-18

### Added

- **Engine** now owns `SceneManager` and provides `switch_scene()`. The engine lives outside the ECS `World`, so `World::clear()` during scene switching no longer destroys the scene manager — consecutive switches work correctly.

### Changed

- `Engine` in `runa_engine` is no longer a stub: it holds `scene_manager: SceneManager` and a `switch_scene(&mut self, name, &mut World)` method.

## [0.7.2] - 2026-07-18

### Changed

- **Systems migration:** Builtin systems (`cursor_interaction`, `audio_system`, `eventbus_system`, `sprite_animator_system`) moved from `runa_engine::systems` to `runa_core::systems::builtin`. `runa_engine` is now a clean umbrella crate again.

## [0.7.1-alpha.2] - 2026-07-18

### Added

- **Rich Text:** `<b>bold</b>`, `<color=#rrggbb>text</color>` in `add_text()`
- **Scene system:** `Scene` trait, `SceneManager`, `#[scene]` auto-registration
- **Save/Load:** `SaveData` + `SceneManager::save_to_file()` / `load_from_file()`
- **World::clear():** new method for clearing the world on scene switching

### Changed

- **Rendering:** `UiText` accepts `segments: Vec<RichTextSegment>` for per-segment styling

## [0.7.1-alpha.2] - 2026-07-17

### Added

- **SceneManager and system**

## [0.7.1-alpha.1] - 2026-07-17

### Added

- **SceneManager:** Added `Scene`, `SceneManager` and `SceneData` to save and construct scenes.
  Implemented for comfortable code-first work.

### Fixed

- **Components:** Now all components are fully implemented through ECS.

## [0.7.0-alpha.4] - 2026-07-15

### Fixed

- **clippy:** Fixed some `cargo clippy` errors.

## [0.7.0-alpha.3] - 2026-07-15

### Fixed

- **clippy:** Fixed some `cargo clippy` errors.

## [0.7.0-alpha.2] - 2026-07-15

### Fixed

- **examples:** added `sandbox_soundtest` and `sandbox_ui` for debug.

## [0.7.0-alpha.1] - 2026-07-15

### Added

- **ECS camera integration:** `Camera` is now an ECS component — spawn it with `world.spawn((Camera::new_orthographic(...),))`.
- **`RunaApp::run_with_world(config, world)`:** third `camera` parameter removed; camera lives in the ECS world.

### Changed

- **OCS completely deleted:** `World`, `Object`, `Script`, `ScriptContext`, `Component` trait, `codefirst`, `InteractionSystem` — all removed.
- **`App` struct simplified:** no more `world_rc`, `camera`, `active_camera_set`, `camera_matrix_override`.
- **Rendering now queries ECS directly:** `render_ecs_sprites()` and `render_ecs_meshes()` iterate `ecs_world.query::<(R<Transform>, R<SpriteRenderer>)>()`.
- **`#[system]` API:** takes `&mut runa_ecs::World` directly — no `Script` / `ScriptContext`.
- **`#[derive(Component)]` removed from `runa_macros`:** only `#[system]` remains.
- **Examples rewritten:** sandbox (sprite + WASD movement), sandbox_3d (rotating cubes), sandbox_soundtest, sandbox_ui — all use new ECS-only API.
- **`AGENTS.md` deleted:** cross-references replaced with "See ROADMAP.md".
- **README.md updated:** quick start, API references, and crate list reflect new ECS-only world.

### Removed

- Entire `runa_core::ocs` module (world, object, script, commands).
- `runa_core::codefirst`, `runa_core::components::component` (old `Component` trait).
- `runa_core::systems::interaction_system`.
- `runa_engine::sprite!` macro, old ECS re-exports from `runa_engine`.
- `Engine::create_world()` — `engine.rs` is now a no-op struct.

## [0.6.1-alpha.1] - 2026-07-09

### Added

- **OCS**: Added `with_child` to add child object when constructing object.

### Changed

- **`SpriteRenderer::new(texture)`:** Remove previously required parameter `texture_path`:
  `SpriteRenderer::new(texture, texture_path)` -> `SpriteRenderer::new(texture)`.  
  `texture_path` is extracted from the handle's metadata.

### Features

- Ui debug overlays, font_scale fix, world-to-screen y-axis fix, with_size for auto-layout, DebugLine primitive

## [0.6.0-alpha.4] - 2026-07-02

### Added

- **Mouse input:** `is_mouse_button_just_released` added.

### Fixed

- **Documentation:** Updated the documentation to align with the latest data.
  - [Creating a 2d game](docs\tutorials\getting-started\creating-a-2d-game.md)
  - [Creating a 3d game](docs\tutorials\getting-started\creating-a-3d-game.md)

## [0.6.0-alpha.3] - 2026-07-02

### Added

- **Mouse input:** `is_mouse_button_just_released`, `mouse_position()`, `mouse_delta()`, `mouse_scroll_delta()`
  — free functions and `InputState` methods. Unity-style `GetMouseButtonUp` support.

### Changed

- `ElementState::Released` no longer removes from `mouse_buttons_just_pressed`.

## [0.6.0-alpha.2] - 2026-07-02

### Added

- **Code-first API:** `Bundle` trait, `spawn_bundle((A, B, C))`, `query_components::<T>()`
  — spawn and query entities without registration.
- **`Color` struct:** RGBA `f32`, HSV/HSL/hex conversions, named constants,
  linear/gamma, premultiply, blend. Replaces `Vec3` for all color-typed fields.
- **`math` module:** `lerp`, `smooth_step`, `smooth_damp`, `ease_in/out/*`,
  `move_towards`, `remap`, `inverse_lerp`, `lerp_angle`, `LerpExt` trait for
  `f32`/`Vec2`/`Vec3`/`Vec4`.
- **`SpriteRenderer::from_path(path)`:** lazy texture loading from `texture_path`
  on first render (via `OnceLock`).
- **`sprite!("path")` macro:** compile-time validated, returns `SpriteRenderer`
  with texture loaded — no path duplication.
- **`runa_engine::prelude`:** glob import for all common types.
- **`Node` UI helpers:** `with_margin`, `with_padding` builders.

### Changed

- **Registration system removed entirely:**
  `RunaComponent`, `RunaScript`, `RunaArchetype`, `RunaObjectDef` derives,
  `TypeRegistry`, `RuntimeRegistry`, `engine.register_*`, `ObjectBuilder`,
  `WorldSpawnArg` — all deleted. Plain `#[derive(Component)]` + `world.spawn_bundle`
  is the new standard.
- **`Component` trait simplified:** no `SerializedFieldAccess` supertrait,
  no `runtime_kind`/`runtime_type_name`. Requires `Any + Send + Sync + 'static`.
- **Closures in components** (`CursorInteractable`, `UiNode`) wrapped in `Mutex`
  for `Sync`.
- **`runa_project` / `runa_editor` excluded from workspace** — will be
  refactored for a later editor release.
- **Examples rewritten:** use `Color::BLACK`, `world.spawn_bundle`,
  `#[derive(Component)]`, `SpriteRenderer::from_path`.
- **Docs updated:** README, tutorials, architecture docs reflect new API.

### Removed

- All registration plumbing (`registry.rs`, `Runa*` derives, `register_*` methods).
- `WorldSpawnArg` trait and all `spawn_archetype`/`spawn_def` methods.
- `ObjectBuilder`, `ObjectComponentInfo`, `WorldExt` trait.
- Old test files `typed_archetypes.rs`, `serialized_fields.rs`.
- `runa_engine` dependency on `runa_project`.

## [0.6.0-alpha.1] - 2026-06-29

### Added

- **UI layout:** margin support for free positioning (inset from anchor edge)
  and vbox/hbox auto-layout (space reservation + content offset).
- **UI builder:** `with_fill()` convenience method (sets `Anchor::Stretch`).
- **UI interaction:** `process_interaction()` called each frame with
  `left_just_down` detection, fixing click/hover/drag/slider on `CanvasSpace::Camera`.
- **UI rendering:** `screen_scale` field fixes coordinate mismatch between
  layout (virtual space) and render (screen pixels).
- **ECS:** O(1) `ObjectId` lookup and `Vec`-based component storage.
- **GPU instancing:** instanced sprite/tile rendering with mesh cache and
  persistent uniform buffers.
- **Post-processing:** screen effects pipeline (fade, vignette, RGB shift, tint).
- **Console plugin system:** autocomplete, FPS overlay, world path queries.
- **User event system:** `emit_event` / `subscribe_to_event` API.
- **`.runa3d` format:** model format support with mesh loading.
- **Editor:** UI panels (content browser, inspector, hierarchy), editor
  commands, viewport object picking, hierarchy drag-and-drop parenting,
  inline rename, multi-selection.

### Changed

- **Strategic pivot:** editor feature development frozen until v0.10.
  All effort goes into code-first core engine API.
- **Rendering commands:** refactored `RenderCommands` for cleaner GPU batching.
- **Script lifecycle:** `&mut World` now passed to `ScriptContext`.
- **Tilemap:** coordinate conversion, dynamic resize, frustum-culled batch rendering.
- **Camera:** `orthographic_size` used as virtual layout space for `CanvasSpace::Camera`.
- ROADMAP.md restructured: editor deferred, added code-first DX goals.

### Fixed

- **UI auto-layout:** containers inside vbox/hbox no longer take full virtual
  space — uses minimum content-based sizes, fixing off-screen elements.
- **UI coordinate space:** hit-testing and slider-drag now operate in virtual
  layout space, matching `computed.rect`.
- **Editor:** crashes during Play/world save from stale object-to-world pointers.
- **Editor:** hierarchy traversal hardened against cycles and invalid parent links.
- **Clippy warnings:** redundant fields, unused vars, `div_ceil`, `map_entry`, various casts.

## [0.5.1-alpha.1] - 2026-04-26

### Added

- Added safer editor Play launch logging for diagnosing project startup failures.
- Added hierarchy expand/collapse, inline rename, child object creation, and multi-selection.
- Added viewport object picking and configurable viewport component icon size.

### Fixed

- Fixed editor crashes during Play/world save caused by stale object-to-world pointers after replacing the runtime world.
- Hardened object hierarchy traversal against cycles and invalid parent links.
- Fixed hierarchy drag-and-drop scrolling/hover behavior and collapsed child rendering.

## [0.5.0-alpha.1] - 2026-04-26

### Added

- Added runtime `WorldAtmosphere` with ambient lighting and world-space solid/gradient background rendering.
- Added `DirectionalLight` and `PointLight` runtime components with editor icons and inspector color picker support.
- Added a minimal forward lighting path for 3D meshes with ambient, directional, point, emission, and vertex color inputs.
- Added runtime object hierarchy data: objects can now have a parent and any number of children.
- Added hierarchy drag-and-drop parenting in the editor.
- Added world-transform rendering so parent transforms affect child objects while child transform fields remain local.
- Added a Directional Light direction arrow overlay in the editor viewport.

### Changed

- Mesh rendering now consumes a minimal material shape prepared for future `.runa3d` materials.
- The editor hierarchy now displays root objects and nested children instead of a flat-only list.
- World files now serialize atmosphere data and parent links with versioned world asset data.

### Notes

- No release tag has been created for this version yet.

## [0.4.0-alpha.1] - 2026-04-24

### Added

- **Runtime lifecycle**
  - Added `late_update()` to the script lifecycle
  - `World::update()` now runs `update()` for all objects first and `late_update()` in a second pass

- **Runtime/editor type metadata**
  - Added serialized field metadata for runtime-owned components and scripts
  - Added support for generic serialized component/script storage for editor-facing project data
  - Added project metadata snapshots for archetypes, components, and scripts

- **Editor asset support**
  - Added editor-only SVG icon loading with PNG fallback
  - Added component icon support in the viewport and inspector
  - Added `folder-empty` handling and root-folder navigation in the content browser
  - Added PNG-first component icon loading for sharper editor previews

- **Editor workflow**
  - Added runtime-registry-driven object creation for:
    - empty objects
    - archetypes
    - components
    - scripts
  - Added inspector separation for `Object`, `Transform`, `Components`, and `Scripts`
  - Added bottom inspector actions for adding components and scripts
  - Added manifest-backed `Project Settings` for `RunaAppConfig` values used by `Play In Window`
  - Added `Build Settings` and `Build Game` flow for cargo-driven project builds
  - Added `Content Browser -> Live Rust` split between script-file and archetype-file creation
  - Added a separate Tile Palette window with 16x16 atlas tile previews
  - Added viewport tile painting with `None`, `Paint`, and `Erase` modes

### Changed

- **Camera API**
  - `viewport_size` was removed from public camera constructors
  - `Camera::new_orthographic(width, height)` now stores world-visible size directly
  - `viewport_size` is now runtime-owned state updated by the app/editor from the actual render target size

- **Orthographic camera behavior**
  - Orthographic projection now uses a consistent `look_at_rh` view path
  - Orthographic camera orientation was aligned to `-Z`
  - Orthographic visible area, editor overlays, and `screen_to_world()` now use the same aspect-aware visible size model

- **Editor architecture**
  - `runa_editor` was split into smaller `editor_app` modules for UI, viewport, project flow, world operations, helpers, and placeables
  - Inspector rows now follow a unified `label on the left / controls on the right` layout
  - Mesh renderer editing now supports selecting built-in meshes from the inspector
  - Tilemap editing now uses runtime `Tilemap` data directly instead of manual coordinate inputs

- **Examples and scaffolding**
  - Project scaffolding now generates typed-archetype-first runtime bootstrap
  - Project scaffolding now writes manifest-backed window config and Windows release subsystem setup
  - Bundled examples were updated to the newer camera constructors and typed archetype flow
  - `sandbox` camera follow now runs in `late_update()`

- **Rendering components**
  - Added `SpriteAnimator` as a built-in runtime component for grid-based sprite sheet playback
  - Added `Sorting` as a built-in runtime component for 2D render ordering
  - Added tile atlas and `pixels_per_unit` support to tilemap rendering

### Fixed

- Editor viewport color mismatch caused by incorrect sRGB handling when sampling offscreen targets through `egui`
- Orthographic camera rendering issues related to aspect handling, direction, and visible bounds
- `screen_to_world()` drift and cursor interaction issues at wide resolutions/fullscreen
- Runtime/editor mismatch when using orthographic cameras with resized viewports
- Editor/project metadata refresh now updates archetype/component/script lists from project bootstrap metadata
- Project scripts and serialized fields are now visible again in editor-backed inspection flows
- Serialized script/component overrides now reapply onto existing runtime instances during world loading
- Archetype-backed world objects now preserve serialized script/component override data through editor reconstruction
- Final game rendering now matches source sprite colors more closely than the old editor viewport path
- Fixed component picker icon mismatches, including `AudioListener` and `Sorting`
- Fixed nested inspector text-edit focus loss for `SpriteAnimator` clips and `Tilemap` layers
- Fixed tilemap/sprite draw ordering so later-spawned 2D objects no longer always render above earlier objects

## [0.3.0-alpha.1] - 2026-04-20

### Added

- **Typed archetype API**
  - Added `ArchetypeKey` and `RunaArchetype`
  - Added typed registration via `engine.register_archetype::<T>()`
  - Added typed spawning via `world.spawn_archetype::<T>()`
  - Added `#[derive(RunaArchetype)]` with deterministic snake_case keys
  - Added optional `#[runa(name = \"...\")]` override for archetype names

- **Automatic built-in type registration**
  - `Engine::new()` now registers built-in runtime components automatically
  - Added registry metadata source tracking for built-in vs user registrations
  - Added generic user-type registration via `engine.register::<T>()` for derived components and scripts

- **Runtime registry metadata**
  - Added built-in/user origin metadata for registered components, scripts, and archetypes
  - Exposed queries for built-in and user type sets to support tooling and editor work

- **Documentation and examples refresh**
  - Updated README and tutorials to use the typed archetype flow
  - Updated bundled examples to remove string-based archetype usage from gameplay code
  - Added tests covering typed archetypes and automatic built-in registration

### Changed

- **Breaking:** Archetype registration is now type-driven in normal gameplay code
  - `engine.register_archetype_named(...)` is no longer the primary API
  - `world.spawn_archetype_by_name(...)` remains as a secondary path for tooling, serialization, and editor integration

- **Breaking:** Engine built-in components no longer require user registration
  - User bootstrap code should register only game-specific components, scripts, and archetypes

- **Runtime architecture**
  - Continued the object-first OCS/runtime cleanup around `World`, `Object`, `ScriptContext`, deferred commands, and runtime-owned registry metadata
  - Kept editor/tooling-oriented string lookup as an internal/secondary mechanism instead of a gameplay-facing default

### Fixed

- Reduced fragile setup around engine bootstrap by removing required manual registration of core engine components
- Removed string-only archetype entry points from bundled gameplay examples
- Brought primary docs in line with the typed archetype and explicit registration model

## [0.2.0-alpha.2] - 2026-03-27

### Added

- **Unified Camera System**
  - New `Camera` component supporting both 2D orthographic and 3D perspective projections
  - `Camera::new_orthographic()` - Simple 2D camera setup
  - `Camera::new_perspective()` - Full 3D camera with position, target, FOV
  - Automatic aspect ratio correction for proper rendering
  - `screen_to_world()` conversion for accurate mouse input

- **Documentation**
  - Complete 2D game creation guide
  - Complete 3D game creation guide with FPS controller
  - Updated tutorials README with camera system documentation
  - Quick start guides for both 2D and 3D development paths

- **Input System**
  - Proper camera integration for `get_mouse_world_position()`
  - Aspect ratio correction in screen-to-world conversion
  - Fixed cursor interaction with correct world coordinates

### Changed

- **Breaking:** Camera component refactored
  - `Camera2D` and `Camera3D` deprecated in favor of unified `Camera`
  - Old components remain for backward compatibility with deprecation warnings
  - Migration path: Replace `Camera2D::new()` with `Camera::new_orthographic()`
  - Migration path: Replace `Camera3D { ... }` with `Camera::new_perspective()`

- **Rendering Pipeline**
  - Fixed depth-stencil attachment for sprite pipeline compatibility
  - Proper depth buffer handling for mixed 2D/3D scenes
  - Single render pass for all objects (no more multiple submit calls)
  - Improved performance with batched rendering

- **Interaction System**
  - `InteractionSystem` now updates before scripts for accurate hover state
  - Fixed `CursorInteractable` with proper camera integration
  - Mouse drag now works correctly with aspect ratio correction

### Fixed

- Black screen in 2D scenes (ortho_size now properly calculated)
- Mouse position offset in `CursorInteractable` (aspect ratio correction)
- Depth-stencil format mismatch in sprite rendering
- Camera viewport size not updating from active camera
- Input system using wrong camera for world position

### Deprecated

- `Camera2D` - Use `Camera::new_orthographic()` instead
- `Camera3D` - Use `Camera::new_perspective()` instead

## [0.2.0-alpha.1] - 2026-03-26

### Added

- **3D Camera System**
  - `Camera3D` component with perspective projection
  - `ActiveCamera` marker component for explicit camera selection
  - Automatic camera fallback: ActiveCamera → First Camera3D → Warning
  - Safe rendering when no camera present (black screen, no crash)

- **Cursor Control API**
  - `input_system::show_cursor()` - Show/hide cursor
  - `input_system::lock_cursor()` - Lock/unlock cursor to window
  - `input_system::set_cursor_mode()` - Combined cursor control
  - Global access from anywhere in scripts

- **3D Sandbox Example**
  - `sandbox_3d` - First-person camera controller
  - WASD movement + Space/Ctrl vertical movement
  - Mouse look with locked cursor (right-click toggle)
  - Inverted Y-axis for FPS-style control

- **Input Improvements**
  - `get_mouse_delta()` now uses `DeviceEvent::MouseMotion`
  - Works correctly when cursor is locked
  - No more input lag or single-frame issues

### Changed

- **Breaking:** Camera system now requires explicit camera component
  - Removed automatic default camera creation
  - Add `Camera3D` or `Camera2D` component to enable rendering
  - Use `ActiveCamera` marker for explicit camera selection

- **Breaking:** `AudioSource::play()` API
  - Removed `world` parameter from `Script::update()`
  - Audio playback via `audio.play()` instead of `world.play_sound()`
  - `play_on_awake` flag for automatic playback

- Version bumped to 0.2.0-alpha.1 (3D rendering milestone)

### Documentation

- Updated README.md with 3D camera examples
- Added ActiveCamera usage guide
- Updated cursor control documentation
- Added troubleshooting for "No camera found" warning

## [0.1.3-alpha.1] - 2026-03-26

### Added

- **3D Spatial Audio System**
  - `AudioListener` component for camera/player
  - Distance-based volume attenuation
  - Stereo panning simulation
  - `stereo_separation` parameter

- **AudioSource Improvements**
  - `play_on_awake` flag
  - `play()` and `stop()` methods
  - `min_distance` and `max_distance` controls

- **sandbox_soundtest** example for audio testing

### Changed

- `Script::update()` signature simplified (removed `world` parameter)
- Audio playback via `AudioSource::play()` component method

## [0.1.0]

### Added

- Initial project structure with workspace setup
- Core OCS system (`World`, `Object`, components)
- `Transform` component (mandatory for all objects)
- `Script` trait with lifecycle methods (`construct`, `start`, `update`)
- Global `Input` API for keyboard/mouse access anywhere in code
- 2D rendering pipeline with sprite batching (1000+ objects support)
- Tilemap component with negative coordinate support and texture batching
- `CursorInteractable` component for mouse interaction with objects
- Basic audio system using `rodio` (play/stop sounds)
- Experimental 3D mesh pipeline with depth buffer and instancing
- Camera2D with aspect ratio correction and screen-to-world conversion
- Fullscreen toggle (F11)

### Fixed

- Vertex buffer overwriting causing texture flickering in tilemaps
- Mouse world position calculation with aspect ratio correction
- Bind group caching for 10x rendering performance boost
- Z-fighting prevention in 3D pipeline (proper near/far planes)

### Changed

- Removed `input()` method from `Script` trait (replaced with global `Input` API)
- Unified texture handling: `Arc<TextureAsset>` instead of `Handle`
- Renderer now uses single vertex buffer with offsets for all draw calls
- Camera matrix calculation inverted Y for proper screen coordinates

### Deprecated

- None

### Removed

- None

### Security

- None
