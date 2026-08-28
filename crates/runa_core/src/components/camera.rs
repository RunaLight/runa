use glam::{Mat4, Vec2, Vec3};
use runa_macros::Scriptable;
use runa_script_api::luau::{FromLua, IntoLua, LuaRef, Result, Value};

use super::Transform;

/// Camera projection type.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ProjectionType {
    /// Orthographic projection for 2D rendering.
    #[default]
    Orthographic,
    /// Perspective projection for 3D rendering.
    Perspective,
}

impl<'lua> IntoLua<'lua> for ProjectionType {
    fn into_lua(self, lua: LuaRef<'lua>) -> Result<Value<'lua>> {
        let s = match self {
            ProjectionType::Orthographic => "Orthographic",
            ProjectionType::Perspective => "Perspective",
        };
        s.into_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for ProjectionType {
    fn from_lua(value: Value<'lua>, lua: LuaRef<'lua>) -> Result<Self> {
        if let Value::Nil = value {
            return Ok(Self::default());
        }
        let s: String = String::from_lua(value, lua)?;
        match s.as_str() {
            "Perspective" => Ok(ProjectionType::Perspective),
            _ => Ok(ProjectionType::Orthographic),
        }
    }
}

/// A shared camera component with 2D and 3D support.
#[derive(Debug, Clone, Copy, Scriptable)]
#[script(crate = "::runa_script_api")]
pub struct Camera {
    /// Camera position in world space.
    pub position: Vec3,
    /// Look target for 3D cameras.
    pub target: Vec3,
    /// Up direction for 3D cameras.
    pub up: Vec3,

    /// Projection mode.
    pub projection: ProjectionType,

    // Orthographic projection parameters (2D)
    /// Orthographic camera size as width and height.
    pub orthographic_size: Vec2,
    /// Near clipping plane.
    pub near: f32,
    /// Far clipping plane.
    pub far: f32,

    // Perspective projection parameters (3D)
    /// Field of view in radians for 3D cameras.
    pub fov: f32,

    /// Render viewport size.
    #[script(skip)]
    pub viewport_size: (u32, u32),
}

impl Camera {
    /// Creates a new orthographic camera for 2D rendering.
    ///
    /// # Arguments
    /// * `width` - Visible width
    /// * `height` - Visible height
    pub fn new_orthographic(width: f32, height: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            target: Vec3::NEG_Z,
            up: Vec3::Y,
            projection: ProjectionType::Orthographic,
            orthographic_size: Vec2::new(width.max(f32::EPSILON), height.max(f32::EPSILON)),
            near: -1000.0,
            far: 1000.0,
            fov: 0.0, // Unused for orthographic projection
            viewport_size: (1, 1),
        }
    }

    /// Creates a new perspective camera for 3D rendering.
    ///
    /// # Arguments
    /// * `position` - Camera position
    /// * `target` - Look target
    /// * `up` - Up direction
    /// * `fov` - Field of view in radians
    /// * `near` - Near clipping plane
    /// * `far` - Far clipping plane
    pub fn new_perspective(
        position: Vec3,
        target: Vec3,
        up: Vec3,
        fov: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position,
            target,
            up,
            projection: ProjectionType::Perspective,
            orthographic_size: Vec2::new(320.0, 180.0), // Useful fallback when switching to ortho in tools
            near,
            far,
            fov: fov.to_radians(),
            viewport_size: (1, 1),
        }
    }

    /// Returns the view-projection matrix.
    pub fn matrix(&self) -> Mat4 {
        match self.projection {
            ProjectionType::Orthographic => self.ortho_matrix(),
            ProjectionType::Perspective => self.perspective_matrix(),
        }
    }

    pub fn resolved_with_transform(&self, transform: Option<&Transform>) -> Self {
        let Some(transform) = transform else {
            return *self;
        };

        let position = transform.position + transform.rotation * self.position;
        let target = transform.position + transform.rotation * self.target;
        let up = (transform.rotation * self.up).normalize_or_zero();

        Self {
            position,
            target,
            up: if up.length_squared() > 0.0 {
                up
            } else {
                Vec3::Y
            },
            ..*self
        }
    }

    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize_or_zero()
    }

    pub fn ortho_visible_size(&self) -> Vec2 {
        let viewport_width = self.viewport_size.0.max(1) as f32;
        let viewport_height = self.viewport_size.1.max(1) as f32;
        let base_width = self.orthographic_size.x.max(f32::EPSILON);
        let base_height = self.orthographic_size.y.max(f32::EPSILON);
        let base_aspect = base_width / base_height;
        let viewport_aspect = viewport_width / viewport_height;

        if viewport_aspect >= base_aspect {
            Vec2::new(base_height * viewport_aspect, base_height)
        } else {
            Vec2::new(base_width, base_width / viewport_aspect.max(f32::EPSILON))
        }
    }

    /// Returns the orthographic projection matrix.
    fn ortho_matrix(&self) -> Mat4 {
        let visible_size = self.ortho_visible_size();
        let half_width = visible_size.x * 0.5;
        let half_height = visible_size.y * 0.5;
        let proj = Mat4::orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            self.near,
            self.far,
        );
        let target = if self.forward().length_squared() <= f32::EPSILON {
            self.position + Vec3::NEG_Z
        } else {
            self.target
        };
        let up = if self.up.length_squared() <= f32::EPSILON {
            Vec3::Y
        } else {
            self.up
        };
        let view = Mat4::look_at_rh(self.position, target, up);

        proj * view
    }

    /// Returns the perspective projection matrix.
    fn perspective_matrix(&self) -> Mat4 {
        let aspect = self.viewport_size.0 as f32 / self.viewport_size.1 as f32;
        let proj = Mat4::perspective_rh(self.fov, aspect, self.near, self.far);
        let view = Mat4::look_at_rh(self.position, self.target, self.up);
        proj * view
    }

    /// Sets the camera position.
    pub fn set_position(&mut self, pos: Vec3) {
        self.position = pos;
    }

    /// Sets the orthographic camera size.
    pub fn set_ortho_size(&mut self, size: Vec2) {
        self.orthographic_size = size;
    }

    /// Sets the field of view for a perspective camera.
    pub fn set_fov(&mut self, fov: f32) {
        self.fov = fov;
    }

    /// Returns the aspect ratio.
    pub fn aspect(&self) -> f32 {
        self.viewport_size.0 as f32 / self.viewport_size.1 as f32
    }

    /// Updates the viewport size.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport_size = (width.max(1), height.max(1));
    }

    /// Converts world coordinates to screen pixel coordinates for orthographic cameras.
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        let visible_size = self.ortho_visible_size();
        let half_width = visible_size.x * 0.5;
        let half_height = visible_size.y * 0.5;

        let ndc_x = (world_pos.x - self.position.x) / half_width;
        let ndc_y = (world_pos.y - self.position.y) / half_height;

        let screen_x = (ndc_x + 1.0) * 0.5 * self.viewport_size.0 as f32;
        let screen_y = (1.0 - ndc_y) * 0.5 * self.viewport_size.1 as f32;

        Vec2::new(screen_x, screen_y)
    }

    /// Converts screen coordinates to world coordinates for orthographic cameras.
    pub fn screen_to_world(&self, screen_pos: (f32, f32)) -> Vec2 {
        let (screen_x, screen_y) = screen_pos;
        let (viewport_width, viewport_height) = self.viewport_size;

        // Normalize to NDC
        let ndc_x = (screen_x / viewport_width as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / viewport_height as f32) * 2.0;

        let visible_size = self.ortho_visible_size();
        let half_width = visible_size.x * 0.5;
        let half_height = visible_size.y * 0.5;

        let world_x = ndc_x * half_width + self.position.x;
        let world_y = ndc_y * half_height + self.position.y;

        Vec2::new(world_x, world_y)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new_perspective(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::Y,
            75.0_f32.to_radians(),
            0.1,
            1000.0,
        )
    }
}
