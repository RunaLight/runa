use runa_macros::Scriptable;

/// Active camera marker component
///
/// Attach this to an object to make it the active camera.
/// The object should also have either Camera2D or Camera3D component.
///
/// # Example
/// ```rust,ignore
/// // In your camera script's construct:
/// object.add_component(Camera3D { ... });
/// object.add_component(ActiveCamera);
/// ```
#[derive(Default, Clone, Copy, Debug, Scriptable)]
pub struct ActiveCamera;
