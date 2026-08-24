use glam::Vec2;
use runa_asset::{Handle, TextureAsset};
use std::sync::Mutex;

type UiColor = [f32; 4];
type InteractionCallback = Mutex<Box<dyn FnMut(InteractionState) + Send>>;

#[derive(Clone, Copy, Debug, Default)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiRect {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct UiNodeId(pub u32);

pub struct UiNode {
    pub id: UiNodeId,
    pub name: String,
    pub parent: Option<UiNodeId>,
    pub children: Vec<UiNodeId>,
    pub kind: UiNodeKind,
    pub layout: LayoutProps,
    pub style: StyleProps,
    pub computed: ComputedLayout,
    pub visible: bool,
    pub interaction: InteractionState,
    pub interaction_callback: Option<InteractionCallback>,
}

impl UiNode {
    pub fn new(id: UiNodeId, parent: Option<UiNodeId>, kind: UiNodeKind) -> Self {
        Self {
            id,
            name: String::new(),
            parent,
            children: Vec::new(),
            kind,
            layout: LayoutProps::default(),
            style: StyleProps::default(),
            computed: ComputedLayout::default(),
            visible: true,
            interaction: InteractionState::None,
            interaction_callback: None,
        }
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_layout(mut self, layout: LayoutProps) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_style(mut self, style: StyleProps) -> Self {
        self.style = style;
        self
    }

    /// Returns the computed center position of this node
    pub fn center(&self) -> (f32, f32) {
        (self.computed.rect.x, self.computed.rect.y)
    }

    /// Returns the computed bounds (left, top, right, bottom)
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let half_w = self.computed.rect.w * 0.5;
        let half_h = self.computed.rect.h * 0.5;
        (
            self.computed.rect.x - half_w,
            self.computed.rect.y - half_h,
            self.computed.rect.x + half_w,
            self.computed.rect.y + half_h,
        )
    }

    /// Check if a point (in screen coords) is inside this node
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        let (left, top, right, bottom) = self.bounds();
        px >= left && px <= right && py >= top && py <= bottom
    }

    /// Return slider value if node is slider else return None
    pub fn get_slider_value(&self) -> Option<f32> {
        match &self.kind {
            UiNodeKind::Slider(p) => Some(p.value),
            _ => None,
        }
    }
}

pub enum UiNodeKind {
    Container(ContainerKind),
    Image(ImageProps),
    Text(TextProps),
    Slider(SliderProps),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ContainerKind {
    Free,
    HorizontalBox,
    VerticalBox,
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutProps {
    pub anchor: Anchor,
    pub position: Vec2,
    // pub size: UiSize,
    pub min_size: Vec2,
    pub max_size: Vec2,
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,
    pub gap: f32,
}

impl Default for LayoutProps {
    fn default() -> Self {
        Self {
            anchor: Anchor::TopLeft,
            position: glam::Vec2::ZERO,
            min_size: glam::Vec2::ZERO,
            max_size: glam::Vec2::new(f32::INFINITY, f32::INFINITY),
            margin: EdgeInsets::ZERO,
            padding: EdgeInsets::ZERO,
            gap: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StyleProps {
    pub background: Option<UiColor>,
    pub opacity: f32,
    pub z_index: i16,
}

impl Default for StyleProps {
    fn default() -> Self {
        Self {
            background: None,
            opacity: 1.0,
            z_index: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ComputedLayout {
    pub rect: UiRect,
    pub content_rect: UiRect,
}

#[derive(Clone, Copy, Debug)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Stretch,
}

#[derive(Clone, Copy, Debug)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
}

pub use runa_render_api::command::RichTextSegment;

pub struct TextProps {
    pub text: String,
    pub segments: Vec<RichTextSegment>,
    pub font: Option<FontId>,
    pub font_size: u16,
    pub color: [f32; 4],
    pub line_height: Option<f32>,
    pub align: TextAlign,
}

pub struct ImageProps {
    pub texture: Option<Handle<TextureAsset>>,
    pub tint: [f32; 4],
    pub uv: [f32; 4],
}

#[derive(Clone, Copy, Debug, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

pub use runa_render_api::FontId;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum InteractionState {
    #[default]
    None,
    Hovered,
    Pressed,
    Dragging,
    Clicked,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderProps {
    pub value: f32,
    pub min: f32,
    pub max: f32,
}

impl Default for SliderProps {
    fn default() -> Self {
        Self {
            value: 0.5,
            min: 0.0,
            max: 1.0,
        }
    }
}

/// Reusable stylesheet that can be applied to nodes
#[derive(Clone, Debug, Default)]
pub struct StyleSheet {
    pub background: Option<[f32; 4]>,
    pub background_hover: Option<[f32; 4]>,
    pub opacity: Option<f32>,
    pub z_index: Option<i16>,
    pub padding: Option<EdgeInsets>,
    pub margin: Option<EdgeInsets>,
}

impl StyleSheet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_background(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.background = Some([r, g, b, a]);
        self
    }

    pub fn with_background_hover(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.background_hover = Some([r, g, b, a]);
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    pub fn with_z_index(mut self, z: i16) -> Self {
        self.z_index = Some(z);
        self
    }

    pub fn with_padding(mut self, l: f32, t: f32, r: f32, b: f32) -> Self {
        self.padding = Some(EdgeInsets {
            left: l,
            top: t,
            right: r,
            bottom: b,
        });
        self
    }

    pub fn with_margin(mut self, l: f32, t: f32, r: f32, b: f32) -> Self {
        self.margin = Some(EdgeInsets {
            left: l,
            top: t,
            right: r,
            bottom: b,
        });
        self
    }

    pub fn apply_to(&self, node: &mut UiNode) {
        if let Some(bg) = self.background {
            node.style.background = Some(bg);
        }
        if let Some(opacity) = self.opacity {
            node.style.opacity = opacity;
        }
        if let Some(z) = self.z_index {
            node.style.z_index = z;
        }
        if let Some(padding) = &self.padding {
            node.layout.padding = *padding;
        }
        if let Some(margin) = &self.margin {
            node.layout.margin = *margin;
        }
    }
}
