use regex::Regex;

use crate::{
    animation::manager::AnimationManager,
    colors::{Color, GlobalColorImpl},
    core::keybindings::Keybindings,
    effect::manager::EffectManager,
    theme_manager::ThemeManager,
    user_config::{
        BorderStyle, GlobalRuleConfig, MatchKind, MatchStrategy, UserConfig, WindowMatchConfig,
        WindowRuleConfig,
    },
};

/// Stores the complete configuration including global rules, window rules, and keybindings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedConfig {
    /// Global settings applied across all windows.
    pub global_rule: GlobalRule,
    /// Specific rules for individual windows.
    pub window_rules: Vec<WindowRule>,
    /// Application keybindings.
    pub keybindings: Keybindings,
    /// Enables monitoring for changes in the configuration file.
    pub monitor_config_changes: bool,
    /// Enable custom predefined theme
    pub theme: ThemeManager,
}

impl TryFrom<UserConfig> for ParsedConfig {
    type Error = anyhow::Error;

    fn try_from(value: UserConfig) -> Result<Self, Self::Error> {
        let global_rule = GlobalRule::try_from(value.global_rule)?;
        let window_rules = value
            .window_rules
            .iter()
            .map(|rule| WindowRule::try_from(rule.clone()))
            .collect::<Result<Vec<WindowRule>, _>>()?;

        Ok(Self {
            global_rule,
            window_rules,
            keybindings: value.keybindings,
            monitor_config_changes: value.monitor_config_changes,
            theme: value.theme,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlobalRule {
    /// Default width of the window borders.
    pub border_width: i32,
    /// Default offset for the window borders.
    pub border_offset: i32,
    /// Default border radius settings.
    pub border_style: BorderStyle,
    /// Default color for active window borders.
    pub active_color: Color,
    /// Default color for inactive window borders.
    pub inactive_color: Color,
    /// Animation manager for borders.
    pub animation_manager: AnimationManager,
    /// Effect manager for borders.
    pub effect_manager: EffectManager,
    /// Delay (in milliseconds) before applying borders after initialization.
    pub initialize_delay: u32,
    /// Delay (in milliseconds) before applying borders after unminimizing.
    pub unminimize_delay: u32,
}

impl TryFrom<GlobalRuleConfig> for GlobalRule {
    type Error = anyhow::Error;

    fn try_from(value: GlobalRuleConfig) -> Result<Self, Self::Error> {
        let animation_manager = AnimationManager::try_from(value.animations)?;
        let effect_manager = EffectManager::try_from(value.effects)?;
        let active_color = value.active_color.to_color()?;
        let inactive_color = value.inactive_color.to_color()?;

        Ok(Self {
            animation_manager,
            effect_manager,
            active_color,
            inactive_color,
            border_style: value.border_style,
            border_width: value.border_width,
            border_offset: value.border_offset,
            initialize_delay: value.initialize_delay,
            unminimize_delay: value.unminimize_delay,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowRule {
    /// The matching details and settings for a specific type of window.
    pub match_window: WindowMatch,
}

impl TryFrom<WindowRuleConfig> for WindowRule {
    type Error = anyhow::Error;

    fn try_from(value: WindowRuleConfig) -> Result<Self, Self::Error> {
        let match_window = WindowMatch::try_from(value.match_window)?;
        Ok(Self { match_window })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedMatchStrategy {
    Equals(String),
    Contains(String),
    Regex(String),
}

impl ParsedMatchStrategy {
    pub fn is_match(&self, value: &str) -> bool {
        match self {
            ParsedMatchStrategy::Equals(equals) => value
                .to_ascii_lowercase()
                .eq(equals.to_ascii_lowercase().as_str()),
            ParsedMatchStrategy::Contains(contains) => value
                .to_ascii_lowercase()
                .contains(contains.to_ascii_lowercase().as_str()),
            ParsedMatchStrategy::Regex(regex) => Regex::new(regex)
                .map(|re| re.captures(value).is_some())
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowMatch {
    /// Type of match (e.g., title, class, or process).
    pub match_kind: Option<MatchKind>,
    /// Strategy for matching, such as exact match or regex.
    pub match_strategy: Option<ParsedMatchStrategy>,
    /// Color for the border when the window is active.
    pub active_color: Option<Color>,
    /// Color for the border when the window is inactive.
    pub inactive_color: Option<Color>,
    /// Animation settings for the window borders.
    pub animation_manager: Option<AnimationManager>,
    /// Effect settings for the window borders.
    pub effect_manager: Option<EffectManager>,
    /// Style of the border corners.
    pub border_style: Option<BorderStyle>,
    /// Width of the border in pixels.
    pub border_width: Option<i32>,
    /// Offset of the border relative to the window.
    pub border_offset: Option<i32>,
    /// Whether borders are enabled for this match.
    pub enabled: Option<bool>,
    /// Delay (in milliseconds) before applying the border after initialization.
    pub initialize_delay: Option<u32>,
    /// Delay (in milliseconds) before applying the border after unminimizing.
    pub unminimize_delay: Option<u32>,
}

impl TryFrom<WindowMatchConfig> for WindowMatch {
    type Error = anyhow::Error;

    fn try_from(value: WindowMatchConfig) -> Result<Self, Self::Error> {
        let match_strategy = match (value.match_strategy, value.match_value) {
            (Some(kind), Some(value)) => Some(match kind {
                MatchStrategy::Equals => ParsedMatchStrategy::Equals(value),
                MatchStrategy::Contains => ParsedMatchStrategy::Contains(value),
                MatchStrategy::Regex => ParsedMatchStrategy::Regex(value),
            }),
            (None, Some(value)) => Some(ParsedMatchStrategy::Equals(value)),
            _ => None,
        };

        let animation_manager = value
            .animations
            .map(AnimationManager::try_from)
            .transpose()?;

        let effect_manager = value.effects.map(EffectManager::try_from).transpose()?;

        let active_color = value
            .active_color
            .map(|color| color.to_color())
            .transpose()?;

        let inactive_color = value
            .inactive_color
            .map(|color| color.to_color())
            .transpose()?;

        Ok(Self {
            match_strategy,
            animation_manager,
            effect_manager,
            active_color,
            inactive_color,
            match_kind: value.match_kind,
            border_style: value.border_style,
            border_width: value.border_width,
            border_offset: value.border_offset,
            enabled: value.enabled,
            initialize_delay: value.initialize_delay,
            unminimize_delay: value.unminimize_delay,
        })
    }
}
