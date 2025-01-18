use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EffectKind {
    Glow,
    Shadow,
}

impl FromStr for EffectKind {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "glow" => Ok(EffectKind::Glow),
            "shadow" => Ok(EffectKind::Shadow),
            _ => Err("Unknown effect type"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EffectTranslation {
    pub x: f32,
    pub y: f32,
}
