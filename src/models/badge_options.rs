use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct BadgePreferences {
    style: BadgeStyle,
    pub counter: DownloadCount,
    pub label: String,
    #[serde(skip_serializing_if = "Color::is_none")]
    pub label_color: Color,
    #[serde(skip_serializing_if = "Color::is_none")]
    pub color: Color,
}

impl BadgePreferences {
    pub fn option_fields(&self) -> String {
        let mut output = String::new();
        if let Some(style) = self.style() {
            output.push_str(&format!("&style={style}"));
        }
        if let Some(color) = self.label_color.0 {
            output.push_str(&format!("&labelColor={}", color.as_encoded_hex()));
        }
        if let Some(color) = self.color.0 {
            output.push_str(&format!("&color={}", color.as_encoded_hex()));
        }
        output
    }

    #[inline]
    pub fn set_style(&mut self, style: BadgeStyle) {
        self.style = style
    }

    pub fn style(&self) -> Option<BadgeStyle> {
        if let BadgeStyle::Flat = self.style {
            return None;
        }
        Some(self.style)
    }
}

impl Default for BadgePreferences {
    fn default() -> Self {
        BadgePreferences {
            style: BadgeStyle::default(),
            counter: DownloadCount::default(),
            label: String::from("Nexus Downloads"),
            label_color: Color(None),
            color: Color(None),
        }
    }
}

impl Display for BadgePreferences {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Style preferences: ")?;
        writeln!(f, "- Label: {}", self.label)?;
        writeln!(f, "- Tracking: {}", self.counter)?;
        if let Some(style) = self.style() {
            writeln!(f, "- Style: {style}")?;
        }
        if let Some(color) = self.label_color.0 {
            writeln!(f, "- Label color: {}", color.as_hex_color())?;
        }
        if let Some(color) = self.color.0 {
            writeln!(f, "- Color: {}", color.as_hex_color())?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Color(Option<u32>);

impl Color {
    #[inline]
    fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

pub trait EncodedHex {
    fn as_encoded_hex(&self) -> String;
    fn as_hex_color(&self) -> String;
}

impl EncodedHex for u32 {
    #[inline]
    fn as_encoded_hex(&self) -> String {
        format!("%23{self:06x}")
    }
    #[inline]
    fn as_hex_color(&self) -> String {
        format!("#{self:06x}")
    }
}

impl std::str::FromStr for Color {
    type Err = Cow<'static, str>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("default") {
            return Ok(Color(None));
        }

        let hex = s.trim_start_matches('#');

        if hex.len() != 6 {
            return Err("Color must be 6 hex digits".into());
        }

        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Color must contain only hex digits".into());
        }

        u32::from_str_radix(hex, 16)
            .map(|int| Color(Some(int)))
            .map_err(|err| format!("Invalid hex color. {err}").into())
    }
}

#[derive(Deserialize, Serialize, Default, Clone, Copy, Debug, ValueEnum)]
pub enum DownloadCount {
    #[default]
    #[value(alias = "Total")]
    Total,
    #[value(alias = "Unique")]
    Unique,
}

impl Display for DownloadCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DownloadCount::Total => "Total downloads",
                DownloadCount::Unique => "Unique downloads",
            }
        )
    }
}

impl DownloadCount {
    pub fn field_name(&self) -> &'static str {
        match self {
            DownloadCount::Total => "mod_downloads",
            DownloadCount::Unique => "mod_unique_downloads",
        }
    }
}

#[derive(Deserialize, Serialize, Default, Clone, Copy, Debug, ValueEnum)]
pub enum BadgeStyle {
    #[default]
    #[value(alias = "Flat")]
    Flat,
    #[value(aliases(["flatsquare", "flatSquare", "Flat-Square", "flat_square"]))]
    FlatSquare,
    #[value(alias = "Plastic")]
    Plastic,
    #[value(aliases(["forthebadge", "forTheBadge", "For-The-Badge", "for_the_badge"]))]
    ForTheBadge,
    #[value(alias = "Social")]
    Social,
}

impl Display for BadgeStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BadgeStyle::Flat => "flat",
                BadgeStyle::FlatSquare => "flat-square",
                BadgeStyle::Plastic => "plastic",
                BadgeStyle::ForTheBadge => "for-the-badge",
                BadgeStyle::Social => "social",
            }
        )
    }
}
