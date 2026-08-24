mod ui_asset;
mod ui_builder;
mod ui_node;

pub use ui_asset::{
    AnchorAsset, ContainerKindAsset, EdgeInsetsAsset, ImagePropsAsset, LayoutPropsAsset,
    SliderPropsAsset, StylePropsAsset, TextAlignAsset, TextPropsAsset, UiAssetFile, UiNodeAsset,
    UiNodeKindAsset,
};
pub use ui_node::{
    Anchor, ContainerKind, EdgeInsets, FontId, ImageProps, InteractionState, LayoutProps,
    RichTextSegment, SliderProps, StyleProps, StyleSheet, TextAlign, TextProps, UiNode, UiNodeId,
    UiNodeKind, UiRect,
};

pub use ui_builder::UiNodeBuilder;

pub enum CanvasSpace {
    Screen,
    Camera,
    World,
}

/// Parse basic rich-text tags into segments.
///
/// Supported tags:
/// - `<b>...</b>` — bold
/// - `<color=#rrggbb>...</color>` — hex color
/// - `<color=#rrggbbaa>...</color>` — hex color with alpha
pub fn parse_rich_text(input: &str) -> Vec<RichTextSegment> {
    if !input.contains('<') {
        return vec![RichTextSegment {
            text: input.to_string(),
            color: [1.0, 1.0, 1.0, 1.0],
            bold: false,
        }];
    }

    let mut segments: Vec<RichTextSegment> = Vec::new();
    let mut remaining = input;
    let mut current_color = [1.0, 1.0, 1.0, 1.0];
    let mut current_bold = false;

    loop {
        // find next tag
        if let Some(tag_start) = remaining.find('<') {
            // push text before tag
            if tag_start > 0 {
                segments.push(RichTextSegment {
                    text: remaining[..tag_start].to_string(),
                    color: current_color,
                    bold: current_bold,
                });
            }
            remaining = &remaining[tag_start..];

            // find tag close
            if let Some(tag_end) = remaining.find('>') {
                let tag = &remaining[1..tag_end];
                remaining = &remaining[tag_end + 1..];

                if tag == "/b" || tag == "/bold" {
                    current_bold = false;
                } else if tag == "b" || tag == "bold" {
                    current_bold = true;
                } else if tag == "/color" {
                    current_color = [1.0, 1.0, 1.0, 1.0];
                } else if let Some(color_val) = tag.strip_prefix("color=") {
                    let hex = color_val.trim_start_matches('#');
                    current_color = hex_to_color(hex);
                }
                // unknown tag — skip
            } else {
                // malformed — push rest as plain text
                segments.push(RichTextSegment {
                    text: remaining.to_string(),
                    color: current_color,
                    bold: current_bold,
                });
                break;
            }
        } else {
            // no more tags — push remaining text
            if !remaining.is_empty() {
                segments.push(RichTextSegment {
                    text: remaining.to_string(),
                    color: current_color,
                    bold: current_bold,
                });
            }
            break;
        }
    }

    segments
}

fn hex_to_color(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            [
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ]
        }
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}
