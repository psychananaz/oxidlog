use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub journal_cfg: JournalConfig,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct JournalConfig {
    /// Display stored tags after the body instead of in a separate tag field.
    pub inline_tags: bool,
    pub show_time: bool,
    #[serde(default = "default_export_dir")]
    pub export_dir: String,
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            inline_tags: false,
            show_time: false,
            export_dir: default_export_dir(),
        }
    }
}

fn default_export_dir() -> String {
    "exports".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(!config.journal_cfg.inline_tags);
        assert!(!config.journal_cfg.show_time);
        assert_eq!(config.journal_cfg.export_dir, "exports");
        let parsed: Config = toml::from_str("[journal_cfg]\n").unwrap();
        assert_eq!(parsed.journal_cfg.export_dir, config.journal_cfg.export_dir);
    }

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.journal_cfg.inline_tags = true;
        config.journal_cfg.show_time = true;

        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert!(deserialized.journal_cfg.inline_tags);
        assert!(deserialized.journal_cfg.show_time);
    }
}
