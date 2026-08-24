use std::path::Path;
use std::sync::Arc;

use super::{
    Anchor, ContainerKind, EdgeInsets, ImageProps, LayoutProps, SliderProps, StyleProps, TextAlign,
    TextProps, UiNode, UiNodeId, UiNodeKind,
};
use crate::{components::UiRenderer, ui::CanvasSpace};
use runa_asset::{Handle, TextureAsset};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiNodeAsset {
    pub id: u32,
    #[serde(default)]
    pub parent: Option<u32>,
    #[serde(default)]
    pub children: Vec<u32>,
    pub kind: UiNodeKindAsset,
    #[serde(default)]
    pub layout: LayoutPropsAsset,
    #[serde(default)]
    pub style: StylePropsAsset,
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default)]
    pub name: String,
}

fn default_visible() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UiNodeKindAsset {
    Container(ContainerKindAsset),
    Image(ImagePropsAsset),
    Text(TextPropsAsset),
    Slider(SliderPropsAsset),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct SliderPropsAsset {
    #[serde(default)]
    pub value: f32,
    #[serde(default)]
    pub min: f32,
    #[serde(default)]
    pub max: f32,
}

impl Default for SliderPropsAsset {
    fn default() -> Self {
        Self {
            value: 0.5,
            min: 0.0,
            max: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub enum ContainerKindAsset {
    #[default]
    Free,
    HorizontalBox,
    VerticalBox,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImagePropsAsset {
    #[serde(default)]
    pub texture_path: Option<String>,
    #[serde(default)]
    pub tint: [f32; 4],
    #[serde(default)]
    pub uv: [f32; 4],
}

impl Default for ImagePropsAsset {
    fn default() -> Self {
        Self {
            texture_path: None,
            tint: [1.0, 1.0, 1.0, 1.0],
            uv: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TextPropsAsset {
    #[serde(default)]
    pub text: String,
    #[serde(default = "default_font_size")]
    pub font_size: u16,
    #[serde(default)]
    pub color: [f32; 4],
    #[serde(default)]
    pub line_height: Option<f32>,
    #[serde(default)]
    pub align: TextAlignAsset,
}

fn default_font_size() -> u16 {
    16
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub enum TextAlignAsset {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct LayoutPropsAsset {
    #[serde(default)]
    pub anchor: AnchorAsset,
    #[serde(default)]
    pub position: [f32; 2],
    #[serde(default)]
    pub min_size: [f32; 2],
    #[serde(default = "default_max_size")]
    pub max_size: [f32; 2],
    #[serde(default)]
    pub margin: EdgeInsetsAsset,
    #[serde(default)]
    pub padding: EdgeInsetsAsset,
    #[serde(default)]
    pub gap: f32,
}

fn default_max_size() -> [f32; 2] {
    [f32::INFINITY, f32::INFINITY]
}

impl Default for LayoutPropsAsset {
    fn default() -> Self {
        Self {
            anchor: AnchorAsset::TopLeft,
            position: [0.0, 0.0],
            min_size: [0.0, 0.0],
            max_size: [f32::INFINITY, f32::INFINITY],
            margin: EdgeInsetsAsset::ZERO,
            padding: EdgeInsetsAsset::ZERO,
            gap: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub enum AnchorAsset {
    #[default]
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct StylePropsAsset {
    #[serde(default)]
    pub background: Option<[f32; 4]>,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub z_index: i16,
}

fn default_opacity() -> f32 {
    1.0
}

impl Default for StylePropsAsset {
    fn default() -> Self {
        Self {
            background: None,
            opacity: 1.0,
            z_index: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct EdgeInsetsAsset {
    #[serde(default)]
    pub left: f32,
    #[serde(default)]
    pub top: f32,
    #[serde(default)]
    pub right: f32,
    #[serde(default)]
    pub bottom: f32,
}

impl EdgeInsetsAsset {
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
}

impl Default for EdgeInsetsAsset {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiAssetFile {
    #[serde(default = "default_viewport_width")]
    pub viewport_width: f32,
    #[serde(default = "default_viewport_height")]
    pub viewport_height: f32,
    pub nodes: Vec<UiNodeAsset>,
}

fn default_viewport_width() -> f32 {
    1920.0
}

fn default_viewport_height() -> f32 {
    1080.0
}

impl UiAssetFile {
    pub fn empty() -> Self {
        let root = UiNodeAsset {
            id: 0,
            parent: None,
            children: Vec::new(),
            kind: UiNodeKindAsset::Container(ContainerKindAsset::Free),
            layout: LayoutPropsAsset::default(),
            style: StylePropsAsset::default(),
            visible: true,
            name: "Root".to_string(),
        };
        Self {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            nodes: vec![root],
        }
    }

    pub fn from_ui_renderer(renderer: &UiRenderer) -> Self {
        let nodes: Vec<UiNodeAsset> = renderer
            .nodes
            .iter()
            .map(UiNodeAsset::from_ui_node)
            .collect();
        Self {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            nodes,
        }
    }

    pub fn into_ui_renderer(self, project_root: Option<&Path>) -> UiRenderer {
        let mut renderer = UiRenderer::new(CanvasSpace::Screen);
        renderer.nodes = self
            .nodes
            .iter()
            .map(|n| n.to_ui_node(project_root))
            .collect();
        if let Some(first) = self.nodes.first() {
            renderer.root = UiNodeId(first.id);
        }
        renderer.dirty_layout = true;
        renderer
    }
}

impl UiNodeAsset {
    pub fn from_ui_node(node: &UiNode) -> Self {
        Self {
            id: node.id.0,
            parent: node.parent.map(|p| p.0),
            children: node.children.iter().map(|c| c.0).collect(),
            kind: UiNodeKindAsset::from_kind(&node.kind),
            layout: LayoutPropsAsset::from_layout(&node.layout),
            style: StylePropsAsset::from_style(&node.style),
            visible: node.visible,
            name: node.name.clone(),
        }
    }

    pub fn to_ui_node(&self, project_root: Option<&Path>) -> UiNode {
        let mut node = UiNode::new(
            UiNodeId(self.id),
            self.parent.map(UiNodeId),
            self.kind.to_kind(project_root),
        )
        .named(self.name.clone());
        node.children = self.children.iter().map(|c| UiNodeId(*c)).collect();
        node.layout = self.layout.to_layout();
        node.style = self.style.to_style();
        node.visible = self.visible;
        node
    }
}

impl UiNodeKindAsset {
    pub fn from_kind(kind: &UiNodeKind) -> Self {
        match kind {
            UiNodeKind::Container(ck) => UiNodeKindAsset::Container(match ck {
                ContainerKind::Free => ContainerKindAsset::Free,
                ContainerKind::HorizontalBox => ContainerKindAsset::HorizontalBox,
                ContainerKind::VerticalBox => ContainerKindAsset::VerticalBox,
            }),
            UiNodeKind::Image(props) => UiNodeKindAsset::Image(ImagePropsAsset {
                texture_path: None,
                tint: props.tint,
                uv: props.uv,
            }),
            UiNodeKind::Text(props) => {
                let align = match props.align {
                    TextAlign::Left => TextAlignAsset::Left,
                    TextAlign::Center => TextAlignAsset::Center,
                    TextAlign::Right => TextAlignAsset::Right,
                };
                UiNodeKindAsset::Text(TextPropsAsset {
                    text: props.text.clone(),
                    font_size: props.font_size,
                    color: props.color,
                    line_height: props.line_height,
                    align,
                })
            }
            UiNodeKind::Slider(props) => UiNodeKindAsset::Slider(SliderPropsAsset {
                value: props.value,
                min: props.min,
                max: props.max,
            }),
        }
    }

    pub fn to_kind(&self, project_root: Option<&Path>) -> UiNodeKind {
        match self {
            UiNodeKindAsset::Container(ck) => UiNodeKind::Container(match ck {
                ContainerKindAsset::Free => ContainerKind::Free,
                ContainerKindAsset::HorizontalBox => ContainerKind::HorizontalBox,
                ContainerKindAsset::VerticalBox => ContainerKind::VerticalBox,
            }),
            UiNodeKindAsset::Image(props) => {
                let texture = if let (Some(root), Some(path)) = (project_root, &props.texture_path)
                {
                    let full = root.join(path);
                    if let Ok(tex) = TextureAsset::load(&full) {
                        Some(Handle {
                            inner: Arc::new(tex),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                UiNodeKind::Image(ImageProps {
                    texture,
                    tint: props.tint,
                    uv: props.uv,
                })
            }
            UiNodeKindAsset::Text(props) => {
                let align = match props.align {
                    TextAlignAsset::Left => TextAlign::Left,
                    TextAlignAsset::Center => TextAlign::Center,
                    TextAlignAsset::Right => TextAlign::Right,
                };
                UiNodeKind::Text(TextProps {
                    text: props.text.clone(),
                    segments: vec![],
                    font: None,
                    font_size: props.font_size,
                    color: props.color,
                    line_height: props.line_height,
                    align,
                })
            }
            UiNodeKindAsset::Slider(props) => UiNodeKind::Slider(SliderProps {
                value: props.value.clamp(props.min, props.max),
                min: props.min,
                max: props.max,
            }),
        }
    }
}

impl LayoutPropsAsset {
    pub fn from_layout(layout: &LayoutProps) -> Self {
        Self {
            anchor: AnchorAsset::from_anchor(&layout.anchor),
            position: layout.position.to_array(),
            min_size: layout.min_size.to_array(),
            max_size: layout.max_size.to_array(),
            margin: EdgeInsetsAsset {
                left: layout.margin.left,
                top: layout.margin.top,
                right: layout.margin.right,
                bottom: layout.margin.bottom,
            },
            padding: EdgeInsetsAsset {
                left: layout.padding.left,
                top: layout.padding.top,
                right: layout.padding.right,
                bottom: layout.padding.bottom,
            },
            gap: layout.gap,
        }
    }

    pub fn to_layout(&self) -> LayoutProps {
        LayoutProps {
            anchor: self.anchor.to_anchor(),
            position: glam::Vec2::from_array(self.position),
            min_size: glam::Vec2::from_array(self.min_size),
            max_size: glam::Vec2::from_array(self.max_size),
            margin: EdgeInsets {
                left: self.margin.left,
                top: self.margin.top,
                right: self.margin.right,
                bottom: self.margin.bottom,
            },
            padding: EdgeInsets {
                left: self.padding.left,
                top: self.padding.top,
                right: self.padding.right,
                bottom: self.padding.bottom,
            },
            gap: self.gap,
        }
    }
}

impl AnchorAsset {
    pub fn from_anchor(anchor: &Anchor) -> Self {
        match anchor {
            Anchor::TopLeft => Self::TopLeft,
            Anchor::TopCenter => Self::TopCenter,
            Anchor::TopRight => Self::TopRight,
            Anchor::Left => Self::Left,
            Anchor::Center => Self::Center,
            Anchor::Right => Self::Right,
            Anchor::BottomLeft => Self::BottomLeft,
            Anchor::BottomCenter => Self::BottomCenter,
            Anchor::BottomRight => Self::BottomRight,
            Anchor::Stretch => Self::Stretch,
        }
    }

    pub fn to_anchor(&self) -> Anchor {
        match self {
            Self::TopLeft => Anchor::TopLeft,
            Self::TopCenter => Anchor::TopCenter,
            Self::TopRight => Anchor::TopRight,
            Self::Left => Anchor::Left,
            Self::Center => Anchor::Center,
            Self::Right => Anchor::Right,
            Self::BottomLeft => Anchor::BottomLeft,
            Self::BottomCenter => Anchor::BottomCenter,
            Self::BottomRight => Anchor::BottomRight,
            Self::Stretch => Anchor::Stretch,
        }
    }
}

impl StylePropsAsset {
    pub fn from_style(style: &StyleProps) -> Self {
        Self {
            background: style.background,
            opacity: style.opacity,
            z_index: style.z_index,
        }
    }

    pub fn to_style(&self) -> StyleProps {
        StyleProps {
            background: self.background,
            opacity: self.opacity,
            z_index: self.z_index,
        }
    }
}
