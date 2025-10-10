use bevy::prelude::*;
use std::collections::HashMap;

/// Resource that stores the mapping between Material icon names and their glyphs.
#[derive(Resource, Debug)]
pub struct MaterialIcons {
    glyphs: HashMap<String, String>,
}

impl MaterialIcons {
    fn normalize(name: &str) -> String {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let replaced = trimmed
            .replace(['-', ' ', '.'], "_")
            .chars()
            .collect::<Vec<char>>();
        let snake = Self::to_snake_case(&replaced);
        snake.to_lowercase()
    }

    fn to_snake_case(chars: &[char]) -> String {
        let mut result = String::with_capacity(chars.len());
        for (idx, ch) in chars.iter().enumerate() {
            if *ch == '_' {
                if !result.ends_with('_') {
                    result.push('_');
                }
                continue;
            }

            if ch.is_uppercase() {
                if idx > 0 {
                    let prev = chars[idx - 1];
                    let next = chars.get(idx + 1).copied();
                    let should_insert = (prev.is_lowercase() || prev.is_ascii_digit())
                        || next.map(|c| c.is_lowercase()).unwrap_or(false);
                    if should_insert && !result.ends_with('_') {
                        result.push('_');
                    }
                }
                for lower in ch.to_lowercase() {
                    result.push(lower);
                }
            } else {
                result.push(*ch);
            }
        }
        result.trim_matches('_').to_string()
    }

    /// Returns the glyph string associated with a particular icon name.
    pub fn glyph(&self, name: &str) -> Option<&str> {
        if name.is_empty() {
            return None;
        }
        let normalized = Self::normalize(name);
        self.glyphs
            .get(&normalized)
            .map(String::as_str)
            .or_else(|| self.glyphs.get(name).map(String::as_str))
    }
}

impl FromWorld for MaterialIcons {
    fn from_world(_world: &mut World) -> Self {
        let mut glyphs = HashMap::new();
        for line in include_str!("../../assets/fonts/GoogleMaterialIcons.codepoints").lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(name) = parts.next() else { continue };
            let Some(codepoint) = parts.next() else {
                continue;
            };

            if let Ok(value) = u32::from_str_radix(codepoint, 16) {
                if let Some(ch) = char::from_u32(value) {
                    let glyph = ch.to_string();
                    let key = Self::normalize(name);
                    glyphs.entry(key).or_insert(glyph.clone());

                    // Retain the original key as well to support explicitly provided names.
                    glyphs.entry(name.to_string()).or_insert(glyph);
                }
            }
        }

        MaterialIcons { glyphs }
    }
}

#[derive(Clone, Debug)]
pub struct UiIcon {
    name: String,
}

impl UiIcon {
    pub fn new<S: Into<String>>(name: S) -> Self {
        UiIcon { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn glyph<'a>(&self, icons: &'a MaterialIcons) -> Option<&'a str> {
        icons.glyph(&self.name)
    }
}

impl From<&str> for UiIcon {
    fn from(value: &str) -> Self {
        UiIcon::new(value)
    }
}

impl From<String> for UiIcon {
    fn from(value: String) -> Self {
        UiIcon::new(value)
    }
}
