use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::BorderType,
};
use serde::{
    de::{self, IntoDeserializer},
    Deserialize,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_language: String,
    pub history_file: Option<PathBuf>,
    pub theme: Theme,
    pub key_map: KeyMap,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_language: "english200".into(),
            history_file: None,
            theme: Theme::default(),
            key_map: KeyMap::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn matches(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.code == code && modifiers == self.modifiers
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct KeyMap {
    #[serde(deserialize_with = "deserialize_keybinding")]
    pub quit: KeyBinding,
    #[serde(deserialize_with = "deserialize_keybinding")]
    pub restart: KeyBinding,
    #[serde(deserialize_with = "deserialize_keybinding")]
    pub repeat: KeyBinding,
    #[serde(deserialize_with = "deserialize_keybinding")]
    pub practice_missed: KeyBinding,
    #[serde(deserialize_with = "deserialize_keybinding")]
    pub practice_slow: KeyBinding,
    #[serde(deserialize_with = "deserialize_keybinding")]
    pub new_test: KeyBinding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            quit: KeyBinding {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
            },
            restart: KeyBinding {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            },
            repeat: KeyBinding {
                code: KeyCode::Char('t'),
                modifiers: KeyModifiers::NONE,
            },
            practice_missed: KeyBinding {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
            },
            practice_slow: KeyBinding {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::NONE,
            },
            new_test: KeyBinding {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
            },
        }
    }
}

impl KeyMap {
    pub fn check_conflicts(&self) -> Vec<String> {
        let bindings: Vec<(&str, &KeyBinding)> = vec![
            ("quit", &self.quit),
            ("restart", &self.restart),
            ("repeat", &self.repeat),
            ("practice_missed", &self.practice_missed),
            ("practice_slow", &self.practice_slow),
            ("new_test", &self.new_test),
        ];

        let mut seen: HashMap<(KeyCode, KeyModifiers), &str> = HashMap::new();
        let mut conflicts = Vec::new();

        for (name, binding) in &bindings {
            let key = (binding.code, binding.modifiers);
            if let Some(existing) = seen.get(&key) {
                conflicts.push(format!(
                    "Key conflict: '{}' and '{}' are both bound to {}",
                    existing,
                    name,
                    format_keybinding(binding)
                ));
            } else {
                seen.insert(key, name);
            }
        }

        conflicts
    }
}

pub fn format_keybinding(binding: &KeyBinding) -> String {
    let mut parts = Vec::new();
    if binding.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("C".to_string());
    }
    if binding.modifiers.contains(KeyModifiers::ALT) {
        parts.push("A".to_string());
    }
    let key_str = match binding.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        _ => format!("{:?}", binding.code),
    };
    parts.push(key_str);
    parts.join("-")
}

pub fn parse_keybinding(value: &str) -> Result<KeyBinding, String> {
    let parts: Vec<&str> = value.split('-').collect();
    match parts.len() {
        1 => {
            let code = parse_key_code(parts[0])?;
            Ok(KeyBinding {
                code,
                modifiers: KeyModifiers::NONE,
            })
        }
        2 => {
            let modifiers = parse_modifier(parts[0])?;
            let code = parse_key_code(parts[1])?;
            Ok(KeyBinding { code, modifiers })
        }
        _ => Err(format!(
            "Invalid keybinding '{}': expected 'key' or 'modifier-key'",
            value
        )),
    }
}

fn parse_modifier(s: &str) -> Result<KeyModifiers, String> {
    match s {
        "C" => Ok(KeyModifiers::CONTROL),
        "A" => Ok(KeyModifiers::ALT),
        _ => Err(format!(
            "Unknown modifier '{}': expected 'C' (Ctrl) or 'A' (Alt)",
            s
        )),
    }
}

fn parse_key_code(s: &str) -> Result<KeyCode, String> {
    match s {
        "Tab" => Ok(KeyCode::Tab),
        "Backspace" => Ok(KeyCode::Backspace),
        "Enter" => Ok(KeyCode::Enter),
        "Esc" => Ok(KeyCode::Esc),
        "Delete" => Ok(KeyCode::Delete),
        "Space" => Ok(KeyCode::Char(' ')),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            Ok(KeyCode::Char(c))
        }
        _ => Err(format!(
            "Unknown key '{}': expected a single character or one of Tab, Backspace, Enter, Esc, Delete, Space",
            s
        )),
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Theme {
    #[serde(deserialize_with = "deserialize_style")]
    pub default: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub title: Style,

    // test widget
    #[serde(deserialize_with = "deserialize_style")]
    pub input_border: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_border: Style,

    #[serde(deserialize_with = "deserialize_border_type")]
    pub border_type: BorderType,

    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_correct: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_incorrect: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_untyped: Style,

    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_current_correct: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_current_incorrect: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_current_untyped: Style,

    #[serde(deserialize_with = "deserialize_style")]
    pub prompt_cursor: Style,

    // results widget
    #[serde(deserialize_with = "deserialize_style")]
    pub results_overview: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub results_overview_border: Style,

    #[serde(deserialize_with = "deserialize_style")]
    pub results_worst_keys: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub results_worst_keys_border: Style,

    #[serde(deserialize_with = "deserialize_style")]
    pub results_chart: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub results_chart_x: Style,
    #[serde(deserialize_with = "deserialize_style")]
    pub results_chart_y: Style,

    #[serde(deserialize_with = "deserialize_style")]
    pub results_restart_prompt: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            default: Style::default(),

            title: Style::default().add_modifier(Modifier::BOLD),

            input_border: Style::default().fg(Color::Cyan),
            prompt_border: Style::default().fg(Color::Green),

            border_type: BorderType::Rounded,

            prompt_correct: Style::default().fg(Color::Green),
            prompt_incorrect: Style::default().fg(Color::Red),
            prompt_untyped: Style::default().fg(Color::Gray),

            prompt_current_correct: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            prompt_current_incorrect: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            prompt_current_untyped: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),

            prompt_cursor: Style::default().add_modifier(Modifier::UNDERLINED),

            results_overview: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            results_overview_border: Style::default().fg(Color::Cyan),

            results_worst_keys: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            results_worst_keys_border: Style::default().fg(Color::Cyan),

            results_chart: Style::default().fg(Color::Cyan),
            results_chart_x: Style::default().fg(Color::Cyan),
            results_chart_y: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),

            results_restart_prompt: Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        }
    }
}

fn deserialize_keybinding<'de, D>(deserializer: D) -> Result<KeyBinding, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_keybinding(&s).map_err(de::Error::custom)
}

fn deserialize_style<'de, D>(deserializer: D) -> Result<Style, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct StyleVisitor;
    impl de::Visitor<'_> for StyleVisitor {
        type Value = Style;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string describing a text style")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            let (colors, modifiers) = value.split_once(';').unwrap_or((value, ""));
            let (fg, bg) = colors.split_once(':').unwrap_or((colors, "none"));

            let mut style = Style {
                fg: match fg {
                    "none" | "" => None,
                    _ => Some(deserialize_color(fg.into_deserializer())?),
                },
                bg: match bg {
                    "none" | "" => None,
                    _ => Some(deserialize_color(bg.into_deserializer())?),
                },
                ..Default::default()
            };

            for modifier in modifiers.split_terminator(';') {
                style = style.add_modifier(match modifier {
                    "bold" => Modifier::BOLD,
                    "crossed_out" => Modifier::CROSSED_OUT,
                    "dim" => Modifier::DIM,
                    "hidden" => Modifier::HIDDEN,
                    "italic" => Modifier::ITALIC,
                    "rapid_blink" => Modifier::RAPID_BLINK,
                    "slow_blink" => Modifier::SLOW_BLINK,
                    "reversed" => Modifier::REVERSED,
                    "underlined" => Modifier::UNDERLINED,
                    _ => {
                        return Err(E::invalid_value(
                            de::Unexpected::Str(modifier),
                            &"a style modifier",
                        ))
                    }
                });
            }

            Ok(style)
        }
    }

    deserializer.deserialize_str(StyleVisitor)
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct ColorVisitor;
    impl de::Visitor<'_> for ColorVisitor {
        type Value = Color;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a color name or hexadecimal color code")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            match value {
                "reset" => Ok(Color::Reset),
                "black" => Ok(Color::Black),
                "white" => Ok(Color::White),
                "red" => Ok(Color::Red),
                "green" => Ok(Color::Green),
                "yellow" => Ok(Color::Yellow),
                "blue" => Ok(Color::Blue),
                "magenta" => Ok(Color::Magenta),
                "cyan" => Ok(Color::Cyan),
                "gray" => Ok(Color::Gray),
                "darkgray" => Ok(Color::DarkGray),
                "lightred" => Ok(Color::LightRed),
                "lightgreen" => Ok(Color::LightGreen),
                "lightyellow" => Ok(Color::LightYellow),
                "lightblue" => Ok(Color::LightBlue),
                "lightmagenta" => Ok(Color::LightMagenta),
                "lightcyan" => Ok(Color::LightCyan),
                _ => {
                    if value.len() == 6 {
                        let parse_error = |_| E::custom("color code was not valid hexadecimal");

                        Ok(Color::Rgb(
                            u8::from_str_radix(&value[0..2], 16).map_err(parse_error)?,
                            u8::from_str_radix(&value[2..4], 16).map_err(parse_error)?,
                            u8::from_str_radix(&value[4..6], 16).map_err(parse_error)?,
                        ))
                    } else {
                        Err(E::invalid_value(
                            de::Unexpected::Str(value),
                            &"a color name or hexadecimal color code",
                        ))
                    }
                }
            }
        }
    }

    deserializer.deserialize_str(ColorVisitor)
}

fn deserialize_border_type<'de, D>(deserializer: D) -> Result<BorderType, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct BorderTypeVisitor;
    impl de::Visitor<'_> for BorderTypeVisitor {
        type Value = BorderType;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a border type")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            match value {
                "plain" => Ok(BorderType::Plain),
                "rounded" => Ok(BorderType::Rounded),
                "double" => Ok(BorderType::Double),
                "thick" => Ok(BorderType::Thick),
                "quadrantinside" => Ok(BorderType::QuadrantInside),
                "quadrantoutside" => Ok(BorderType::QuadrantOutside),
                _ => Err(E::invalid_value(
                    de::Unexpected::Str(value),
                    &"a border type",
                )),
            }
        }
    }

    deserializer.deserialize_str(BorderTypeVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_basic_colors() {
        fn color(string: &str) -> Color {
            deserialize_color(de::IntoDeserializer::<de::value::Error>::into_deserializer(
                string,
            ))
            .expect("failed to deserialize color")
        }

        assert_eq!(color("black"), Color::Black);
        assert_eq!(color("000000"), Color::Rgb(0, 0, 0));
        assert_eq!(color("ffffff"), Color::Rgb(0xff, 0xff, 0xff));
        assert_eq!(color("FFFFFF"), Color::Rgb(0xff, 0xff, 0xff));
    }

    #[test]
    fn deserializes_styles() {
        fn style(string: &str) -> Style {
            deserialize_style(de::IntoDeserializer::<de::value::Error>::into_deserializer(
                string,
            ))
            .expect("failed to deserialize style")
        }

        assert_eq!(style("none"), Style::default());
        assert_eq!(style("none:none"), Style::default());
        assert_eq!(style("none:none;"), Style::default());

        assert_eq!(style("black"), Style::default().fg(Color::Black));
        assert_eq!(
            style("black:white"),
            Style::default().fg(Color::Black).bg(Color::White)
        );

        assert_eq!(
            style("none;bold"),
            Style::default().add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            style("none;bold;italic;underlined;"),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::ITALIC)
                .add_modifier(Modifier::UNDERLINED)
        );

        assert_eq!(
            style("00ff00:000000;bold;dim;italic;slow_blink"),
            Style::default()
                .fg(Color::Rgb(0, 0xff, 0))
                .bg(Color::Rgb(0, 0, 0))
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::DIM)
                .add_modifier(Modifier::ITALIC)
                .add_modifier(Modifier::SLOW_BLINK)
        );
    }

    #[test]
    fn deserializes_border_types() {
        fn border_type(string: &str) -> BorderType {
            deserialize_border_type(de::IntoDeserializer::<de::value::Error>::into_deserializer(
                string,
            ))
            .expect("failed to deserialize border type")
        }
        assert_eq!(border_type("plain"), BorderType::Plain);
        assert_eq!(border_type("rounded"), BorderType::Rounded);
        assert_eq!(border_type("double"), BorderType::Double);
        assert_eq!(border_type("thick"), BorderType::Thick);
        assert_eq!(border_type("quadrantinside"), BorderType::QuadrantInside);
        assert_eq!(border_type("quadrantoutside"), BorderType::QuadrantOutside);
    }

    #[test]
    fn config_default_has_no_history_file() {
        let config = Config::default();
        assert!(config.history_file.is_none());
    }

    #[test]
    fn config_with_history_file() {
        let toml_str = r#"history_file = "/custom/path/history.csv""#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.history_file,
            Some(PathBuf::from("/custom/path/history.csv"))
        );
    }

    #[test]
    fn config_without_history_file() {
        let toml_str = r#"default_language = "german""#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.history_file.is_none());
        assert_eq!(config.default_language, "german");
    }

    #[test]
    fn parse_simple_char_keybinding() {
        let kb = parse_keybinding("q").unwrap();
        assert_eq!(kb.code, KeyCode::Char('q'));
        assert_eq!(kb.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn parse_special_key_keybinding() {
        let kb = parse_keybinding("Tab").unwrap();
        assert_eq!(kb.code, KeyCode::Tab);
        assert_eq!(kb.modifiers, KeyModifiers::NONE);

        let kb = parse_keybinding("Space").unwrap();
        assert_eq!(kb.code, KeyCode::Char(' '));
        assert_eq!(kb.modifiers, KeyModifiers::NONE);

        let kb = parse_keybinding("Esc").unwrap();
        assert_eq!(kb.code, KeyCode::Esc);
        assert_eq!(kb.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn parse_ctrl_modifier_keybinding() {
        let kb = parse_keybinding("C-r").unwrap();
        assert_eq!(kb.code, KeyCode::Char('r'));
        assert_eq!(kb.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn parse_alt_modifier_keybinding() {
        let kb = parse_keybinding("A-q").unwrap();
        assert_eq!(kb.code, KeyCode::Char('q'));
        assert_eq!(kb.modifiers, KeyModifiers::ALT);
    }

    #[test]
    fn parse_invalid_keybinding() {
        assert!(parse_keybinding("X-q").is_err());
        assert!(parse_keybinding("a-b-c").is_err());
        assert!(parse_keybinding("InvalidKey").is_err());
    }

    #[test]
    fn keybinding_matches_works() {
        let kb = KeyBinding {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        assert!(kb.matches(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!kb.matches(KeyCode::Char('r'), KeyModifiers::NONE));
        assert!(!kb.matches(KeyCode::Char('q'), KeyModifiers::CONTROL));

        let ctrl_r = KeyBinding {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert!(ctrl_r.matches(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert!(!ctrl_r.matches(KeyCode::Char('r'), KeyModifiers::NONE));
    }

    #[test]
    fn keymap_default_values() {
        let km = KeyMap::default();
        assert_eq!(km.quit.code, KeyCode::Char('q'));
        assert_eq!(km.restart.code, KeyCode::Char('r'));
        assert_eq!(km.repeat.code, KeyCode::Char('t'));
        assert_eq!(km.practice_missed.code, KeyCode::Char('p'));
        assert_eq!(km.practice_slow.code, KeyCode::Char('s'));
        assert_eq!(km.new_test.code, KeyCode::Tab);
    }

    #[test]
    fn keymap_from_toml() {
        let toml_str = r#"
[key_map]
quit = "x"
restart = "C-r"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.key_map.quit.code, KeyCode::Char('x'));
        assert_eq!(config.key_map.restart.code, KeyCode::Char('r'));
        assert_eq!(config.key_map.restart.modifiers, KeyModifiers::CONTROL);
        // unspecified keys keep defaults
        assert_eq!(config.key_map.repeat.code, KeyCode::Char('t'));
    }

    #[test]
    fn keymap_conflict_detection() {
        let mut km = KeyMap::default();
        assert!(km.check_conflicts().is_empty());

        // create a conflict: quit and restart both bound to 'q'
        km.restart = KeyBinding {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        let conflicts = km.check_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].contains("quit"));
        assert!(conflicts[0].contains("restart"));
    }

    #[test]
    fn format_keybinding_display() {
        let kb = KeyBinding {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(format_keybinding(&kb), "q");

        let kb = KeyBinding {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
        };
        assert_eq!(format_keybinding(&kb), "C-r");

        let kb = KeyBinding {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(format_keybinding(&kb), "Tab");
    }
}
