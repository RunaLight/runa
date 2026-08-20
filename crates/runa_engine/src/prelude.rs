//! `use runa_engine::prelude::*;` for the most common types.

pub use crate::{
    app::RunaApp,
    app::RunaWindowConfig,
    core::{
        glam::{Mat4, Quat, Vec2, Vec3},
        math::{smooth_damp, smooth_damp_unlimited, smooth_damp_vec3, LerpExt},
    },
    Color, Engine,
};

pub use crate::core::components::{
    ActiveCamera, AlphaMode, AudioListener, AudioSource, Camera, Collider2D, CursorInteractable,
    DirectionalLight, FontId, Material, Mesh, MeshRenderer, PointLight, ProjectionType, Sorting,
    SpriteAnimator, SpriteRenderer, Tilemap, TilemapLayer, TilemapRenderer, Transform, UiRenderer,
    Vertex3D,
};

// pub use crate::core::input::{
//     bind_action, get_mouse_delta, get_mouse_position, get_mouse_scroll_delta,
//     is_action_just_pressed, is_mouse_button_just_released, register_action,
// };

pub use crate::core::EventBus;
pub use crate::core::KeyCode;
pub use crate::core::{console_log, Console, MessageLevel};

pub use crate::scene::{SaveData, Scene, SceneDescriptor, SceneManager};
