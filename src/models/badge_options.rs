use crate::URL_ENCODE_SET;
use clap::ValueEnum;
use percent_encoding::percent_encode;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct BadgePreferences {
    style: BadgeStyle,
    pub count: DownloadCount,
    pub label: String,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(skip_serializing_if = "Color::is_none")]
    pub label_color: Color,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(skip_serializing_if = "Color::is_none")]
    pub color: Color,
}

impl BadgePreferences {
    pub fn option_fields(&self) -> String {
        let mut output = String::new();
        if let Some(style) = self.style() {
            output.push_str(&format!("&style={style}"));
        }
        if let Some(ref color) = self.label_color.0 {
            output.push_str(&format!(
                "&labelColor={}",
                percent_encode(color.as_bytes(), URL_ENCODE_SET)
            ));
        }
        if let Some(ref color) = self.color.0 {
            output.push_str(&format!(
                "&color={}",
                percent_encode(color.as_bytes(), URL_ENCODE_SET)
            ));
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
            label: String::from("Nexus Downloads"),
            style: BadgeStyle::default(),
            count: DownloadCount::default(),
            label_color: Color::default(),
            color: Color::default(),
        }
    }
}

impl Display for BadgePreferences {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Style preferences:")?;
        writeln!(f, "- Label: {}", self.label)?;
        writeln!(f, "- Count: {}", self.count)?;
        writeln!(f, "- Style: {}", self.style)?;
        writeln!(f, "- Label color: {}", self.label_color)?;
        writeln!(f, "- Color: {}", self.color)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Color(Option<String>);

impl Color {
    #[inline]
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_deref().unwrap_or("default"))
    }
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("{err}, Using default color");
            return Ok(Color::default());
        }
    };
    Ok(Color::from_str(&s).unwrap_or_else(|err| {
        eprintln!("'{s}' is not a valid hex color. Using default color.\n{err}");
        Color::default()
    }))
}

impl FromStr for Color {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("default") {
            return Ok(Color(None));
        }

        let hex = s.trim_start_matches('#');

        if hex.len() != 6 {
            return Err("Color must be 6 hex digits");
        }

        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Color must contain only hex digits");
        }

        Ok(Color(Some(format!("#{hex}"))))
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
