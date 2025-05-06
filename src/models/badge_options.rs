use clap::ValueEnum;
use percent_encoding::{AsciiSet, PercentEncode, percent_encode};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

const IMAGE_ALT_TEXT: &str = "Nexus Downloads";

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct BadgePreferences {
    style: BadgeStyle,
    pub format: BadgeFormat,
    pub count: DownloadCount,
    pub label: String,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(skip_serializing_if = "Color::is_none")]
    pub label_color: Color,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(skip_serializing_if = "Color::is_none")]
    pub label_color_light_mode: Color,
    #[serde(deserialize_with = "deserialize_color")]
    #[serde(skip_serializing_if = "Color::is_none")]
    pub color: Color,
}

impl BadgePreferences {
    #[inline]
    pub(crate) fn set_style(&mut self, style: BadgeStyle) {
        self.style = style
    }

    pub(crate) fn style(&self) -> Option<BadgeStyle> {
        (!matches!(self.style, BadgeStyle::Flat)).then_some(self.style)
    }
}

impl Default for BadgePreferences {
    fn default() -> Self {
        BadgePreferences {
            label: String::from("Nexus Downloads"),
            style: BadgeStyle::default(),
            format: BadgeFormat::default(),
            count: DownloadCount::default(),
            label_color: Color::default(),
            label_color_light_mode: Color::default(),
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
        writeln!(f, "- Format: {}", self.format)?;
        writeln!(f, "- Label color: {}", self.label_color)?;
        if self.label_color_light_mode.is_some() && matches!(self.format, BadgeFormat::GithubHtml) {
            writeln!(
                f,
                "- Label color light mode: {}",
                self.label_color_light_mode
            )?;
        }
        writeln!(f, "- Color: {}", self.color)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Color(Option<String>);

impl Color {
    #[inline]
    pub(crate) fn is_none(&self) -> bool {
        self.0.is_none()
    }
    #[inline]
    pub(crate) fn is_some(&self) -> bool {
        self.0.is_some()
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
    pub(crate) fn field_name(&self) -> &'static str {
        match self {
            DownloadCount::Total => "mod_downloads",
            DownloadCount::Unique => "mod_unique_downloads",
        }
    }
}

#[derive(Deserialize, Serialize, Default, Clone, Copy, Debug, ValueEnum)]
pub enum BadgeFormat {
    #[default]
    #[value(alias = "Markdown")]
    Markdown,
    #[value(alias = "Url")]
    Url,
    #[value(alias = "rSt")]
    Rst,
    #[value(aliases = ["AsciiDoc", "asciiDoc", "asciidoc", "Ascii-doc", "ascii_doc"])]
    AsciiDoc,
    #[value(aliases = ["HTML", "Html"])]
    Html,
    #[value(aliases = ["GitHubHtml", "gitHubHtml", "git-hub-html", "Git-hub-html","git_hub_html"])]
    GithubHtml,
}

impl Display for BadgeFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BadgeFormat::Markdown => "markdown",
                BadgeFormat::AsciiDoc => "asciiDoc",
                BadgeFormat::Html => "html",
                BadgeFormat::GithubHtml => "gitHub-Html",
                BadgeFormat::Rst => "rst",
                BadgeFormat::Url => "url",
            }
        )
    }
}

pub struct EncodedFields<'a> {
    json_url: PercentEncode<'a>,
    label_text: PercentEncode<'a>,
    badge_style: Option<BadgeStyle>,
    label_color: Option<PercentEncode<'a>>,
    label_color_light_mode: Option<PercentEncode<'a>>,
    color: Option<PercentEncode<'a>>,
}

impl EncodedFields<'_> {
    fn dynamic_badge_url(
        &self,
        light_mode_valid: bool,
        query: &str,
        ascii_set: &'static AsciiSet,
    ) -> (String, Option<String>) {
        let mut badge_url = format!(
            "https://img.shields.io/badge/dynamic/json?url={}&query={}&label={}",
            self.json_url,
            percent_encode(query.as_bytes(), ascii_set),
            self.label_text
        );

        if let Some(style) = self.badge_style {
            badge_url.push_str(&format!("&style={style}"));
        }

        if let Some(ref color) = self.color {
            badge_url.push_str(&format!("&color={color}"));
        }

        let light_mode_url = self
            .label_color_light_mode
            .as_ref()
            .filter(|_| light_mode_valid)
            .map(|color| format!("{badge_url}&labelColor={color}"));

        if let Some(ref color) = self.label_color {
            badge_url.push_str(&format!("&labelColor={color}"));
        }

        (badge_url, light_mode_url)
    }

    fn dynamic_badge_url_with_link(
        &self,
        query: &str,
        url: &str,
        ascii_set: &'static AsciiSet,
    ) -> (String, Option<String>) {
        let (mut badge_url, _) = self.dynamic_badge_url(false, query, ascii_set);

        badge_url = format!(
            "{badge_url}&link={}",
            percent_encode(url.as_bytes(), ascii_set)
        );
        (badge_url, None)
    }
}

impl<'a> BadgePreferences {
    pub(crate) fn encoded_fields(
        &'a self,
        json_url: &'a str,
        ascii_set: &'static AsciiSet,
    ) -> EncodedFields<'a> {
        let percent_encode_str = |str: &'a str| percent_encode(str.as_bytes(), ascii_set);

        EncodedFields {
            json_url: percent_encode_str(json_url),
            label_text: percent_encode_str(&self.label),
            badge_style: self.style(),
            label_color: self.label_color.0.as_deref().map(percent_encode_str),
            label_color_light_mode: self
                .label_color_light_mode
                .0
                .as_deref()
                .map(percent_encode_str),
            color: self.color.0.as_deref().map(percent_encode_str),
        }
    }
}

impl BadgeFormat {
    fn html_img_tag(badge_url: &str) -> String {
        format!("<img src=\"{badge_url}\" alt=\"{IMAGE_ALT_TEXT}\">")
    }

    pub(crate) fn write_badge(
        &self,
        f: &mut impl std::io::Write,
        ascii_set: &'static AsciiSet,
        encoded_data: &EncodedFields,
        query: &str,
        link_url: &str,
    ) -> std::io::Result<()> {
        let github_html_fmt = matches!(self, Self::GithubHtml);

        let (mut badge_url, mut light_mode_badge_url) =
            if link_url.is_empty() || github_html_fmt || matches!(self, Self::Markdown) {
                encoded_data.dynamic_badge_url(github_html_fmt, query, ascii_set)
            } else {
                encoded_data.dynamic_badge_url_with_link(query, link_url, ascii_set)
            };

        writeln!(f, "```{self}")?;
        match self {
            Self::Markdown => {
                if link_url.is_empty() {
                    writeln!(f, "![{IMAGE_ALT_TEXT}]({badge_url})")?
                } else {
                    writeln!(f, "[![{IMAGE_ALT_TEXT}]({badge_url})]({link_url})")?
                }
            }
            Self::AsciiDoc => writeln!(f, "image:{badge_url}[{IMAGE_ALT_TEXT}]")?,
            Self::Html => writeln!(f, "{}", Self::html_img_tag(&badge_url))?,
            Self::GithubHtml => {
                if !link_url.is_empty() {
                    write!(f, "[")?
                }

                if let Some(light_mode_url) = light_mode_badge_url.as_mut() {
                    write!(
                        f,
                        "<picture>\n    \
                        <source media=\"(prefers-color-scheme: dark)\" srcset=\"{badge_url}\">\n    "
                    )?;
                    std::mem::swap(light_mode_url, &mut badge_url);
                }

                write!(f, "{}", Self::html_img_tag(&badge_url))?;

                if light_mode_badge_url.is_some() {
                    write!(f, "\n</picture>")?;
                }

                writeln!(
                    f,
                    "{}",
                    (!link_url.is_empty())
                        .then(|| format!("]({link_url})"))
                        .unwrap_or_default()
                )?;
            }
            Self::Rst => writeln!(f, ".. image:: {badge_url}\n  :alt: {IMAGE_ALT_TEXT}")?,
            Self::Url => writeln!(f, "{badge_url}")?,
        }
        writeln!(f, "```")
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
