use glam::{Vec3, Vec4};
use runa_macros::Scriptable;

/// RGBA color with f32 components in [0..1] range.
///
/// # Examples
/// ```
/// use runa_core::Color;
///
/// let red = Color::rgb(1.0, 0.0, 0.0);
/// let teal = Color::hex("#008080").unwrap();
/// let (h, s, v) = Color::from_hsv(120.0, 0.5, 0.8).to_hsv();
/// ```
#[derive(Debug, Clone, Copy, Scriptable)]
#[script(crate = "::runa_script_api")]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    // ─── Named constants ───────────────────────────────────────
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    pub const ORANGE: Self = Self::rgb(1.0, 0.6, 0.0);
    pub const GRAY: Self = Self::rgb(0.5, 0.5, 0.5);
    pub const DARK_GRAY: Self = Self::rgb(0.25, 0.25, 0.25);
    pub const LIGHT_GRAY: Self = Self::rgb(0.75, 0.75, 0.75);

    // ─── Constructors ──────────────────────────────────────────

    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    #[inline]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Parse hex color string.
    ///
    /// Supported formats: `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`.
    /// Leading `#` is optional.
    pub fn hex(hex: &str) -> Result<Self, ParseColorError> {
        let s = hex.trim_start_matches('#');
        let (r, g, b, a) = match s.len() {
            3 => {
                let r = u8_from_hex_pair(s, 0, 1)? * 17;
                let g = u8_from_hex_pair(s, 1, 1)? * 17;
                let b = u8_from_hex_pair(s, 2, 1)? * 17;
                (r, g, b, 255)
            }
            4 => {
                let r = u8_from_hex_pair(s, 0, 1)? * 17;
                let g = u8_from_hex_pair(s, 1, 1)? * 17;
                let b = u8_from_hex_pair(s, 2, 1)? * 17;
                let a = u8_from_hex_pair(s, 3, 1)? * 17;
                (r, g, b, a)
            }
            6 => {
                let r = u8_from_hex_pair(s, 0, 2)?;
                let g = u8_from_hex_pair(s, 2, 2)?;
                let b = u8_from_hex_pair(s, 4, 2)?;
                (r, g, b, 255)
            }
            8 => {
                let r = u8_from_hex_pair(s, 0, 2)?;
                let g = u8_from_hex_pair(s, 2, 2)?;
                let b = u8_from_hex_pair(s, 4, 2)?;
                let a = u8_from_hex_pair(s, 6, 2)?;
                (r, g, b, a)
            }
            _ => return Err(ParseColorError::InvalidLength(s.len())),
        };
        Ok(Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        })
    }

    /// Construct from hue‑saturation‑value (HSV).
    ///
    /// * `h` — hue in degrees [0..360)
    /// * `s` — saturation [0..1]
    /// * `v` — value / brightness [0..1]
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h / 60.0;
        let i = h.floor() as i32;
        let f = h - i as f32;
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        let (r, g, b) = match i % 6 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Self { r, g, b, a: 1.0 }
    }

    /// Construct from hue‑saturation‑lightness (HSL).
    ///
    /// * `h` — hue in degrees [0..360)
    /// * `s` — saturation [0..1]
    /// * `l` — lightness [0..1]
    pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = match (h / 60.0).floor() as i32 % 6 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Self {
            r: (r + m).clamp(0.0, 1.0),
            g: (g + m).clamp(0.0, 1.0),
            b: (b + m).clamp(0.0, 1.0),
            a: 1.0,
        }
    }

    // ─── Conversions → glam ────────────────────────────────────

    #[inline]
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.r, self.g, self.b)
    }

    #[inline]
    pub fn to_vec4(self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    #[inline]
    pub fn to_array_4(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[inline]
    pub fn to_array_3(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }

    #[inline]
    pub fn from_vec3(v: Vec3) -> Self {
        Self::rgb(v.x, v.y, v.z)
    }

    #[inline]
    pub fn from_vec4(v: Vec4) -> Self {
        Self::rgba(v.x, v.y, v.z, v.w)
    }

    // ─── Conversions → hex ─────────────────────────────────────

    /// Format as `#RRGGBB`.
    pub fn to_hex(self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    /// Format as `#RRGGBBAA`.
    pub fn to_hex_alpha(self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    // ─── Conversions → HSV / HSL ───────────────────────────────

    /// Returns (hue, saturation, value).
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;

        if delta < 1e-6 {
            return (0.0, 0.0, max);
        }

        let hue = if max == self.r {
            60.0 * (((self.g - self.b) / delta) % 6.0)
        } else if max == self.g {
            60.0 * ((self.b - self.r) / delta + 2.0)
        } else {
            60.0 * ((self.r - self.g) / delta + 4.0)
        };

        (hue_to_0_360(hue), delta / max, max)
    }

    /// Returns (hue, saturation, lightness).
    pub fn to_hsl(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;
        let l = (max + min) / 2.0;

        if delta < 1e-6 {
            return (0.0, 0.0, l);
        }

        let s = delta / (1.0 - (2.0 * l - 1.0).abs());
        let hue = if max == self.r {
            60.0 * (((self.g - self.b) / delta) % 6.0)
        } else if max == self.g {
            60.0 * ((self.b - self.r) / delta + 2.0)
        } else {
            60.0 * ((self.r - self.g) / delta + 4.0)
        };

        (hue_to_0_360(hue), s, l)
    }

    // ─── Manipulation helpers ──────────────────────────────────

    /// Linear → sRGB (gamma‑correct).
    pub fn to_gamma(self) -> Self {
        fn gamma(c: f32) -> f32 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }
        Self {
            r: gamma(self.r),
            g: gamma(self.g),
            b: gamma(self.b),
            a: self.a,
        }
    }

    /// sRGB → linear.
    pub fn to_linear(self) -> Self {
        fn linear(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        Self {
            r: linear(self.r),
            g: linear(self.g),
            b: linear(self.b),
            a: self.a,
        }
    }

    /// Multiply RGB by alpha (premultiply).
    pub fn premultiply(self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }

    /// Blend `other` over `self` (alpha blending).
    pub fn blend_over(self, other: Self) -> Self {
        let a = self.a + other.a * (1.0 - self.a);
        if a < 1e-6 {
            return Self::TRANSPARENT;
        }
        Self {
            r: (self.r * self.a + other.r * other.a * (1.0 - self.a)) / a,
            g: (self.g * self.a + other.g * other.a * (1.0 - self.a)) / a,
            b: (self.b * self.a + other.b * other.a * (1.0 - self.a)) / a,
            a,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl From<(f32, f32, f32)> for Color {
    fn from((r, g, b): (f32, f32, f32)) -> Self {
        Self::rgb(r, g, b)
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Self::rgba(r, g, b, a)
    }
}

impl From<Vec3> for Color {
    fn from(v: Vec3) -> Self {
        Self::from_vec3(v)
    }
}

impl From<Vec4> for Color {
    fn from(v: Vec4) -> Self {
        Self::from_vec4(v)
    }
}

impl From<Color> for Vec3 {
    fn from(c: Color) -> Self {
        c.to_vec3()
    }
}

impl From<Color> for Vec4 {
    fn from(c: Color) -> Self {
        c.to_vec4()
    }
}

impl From<[f32; 3]> for Color {
    fn from(a: [f32; 3]) -> Self {
        Self::rgb(a[0], a[1], a[2])
    }
}

impl From<[f32; 4]> for Color {
    fn from(a: [f32; 4]) -> Self {
        Self::rgba(a[0], a[1], a[2], a[3])
    }
}

// ─── Error type ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseColorError {
    InvalidLength(usize),
    InvalidHexChar(char),
}

impl std::fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength(len) => write!(
                f,
                "invalid hex color length: {len} (expected 3, 4, 6, or 8)"
            ),
            Self::InvalidHexChar(ch) => write!(f, "invalid hex character: '{ch}'"),
        }
    }
}

// ─── Internal helpers ────────────────────────────────────────

fn u8_from_hex_pair(s: &str, start: usize, len: usize) -> Result<u8, ParseColorError> {
    let hex_str = &s[start..start + len];
    u8::from_str_radix(hex_str, 16).map_err(|_| {
        let bad = s.chars().nth(start).unwrap_or('?');
        ParseColorError::InvalidHexChar(bad)
    })
}

fn hue_to_0_360(hue: f32) -> f32 {
    let h = hue % 360.0;
    if h < 0.0 {
        h + 360.0
    } else {
        h
    }
}
