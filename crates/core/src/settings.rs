use camino::Utf8Path;
use config::{builder::DefaultState, Config, ConfigBuilder, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};

/// Shared settings, containing only fields safe for settings inspection.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub log_level: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("could not read config file; check its path, permissions, and UTF-8 encoding")]
    Read(#[source] std::io::Error),
    #[error("could not load settings; check the config file and environment")]
    Load(#[source] config::ConfigError),
    #[error("log_level must be off, error, warn, info, debug, or trace")]
    InvalidLogLevel,
}

impl Settings {
    /// Load defaults, an explicit TOML file, environment, then explicit overrides.
    ///
    /// No config file is discovered automatically. Error sources may contain
    /// source text; adapters must render only the safe error Display, not its chain.
    pub fn load(path: Option<&Utf8Path>, log_level: Option<&str>) -> Result<Self, SettingsError> {
        let mut builder = Config::builder()
            .set_default("log_level", "warn")
            .map_err(SettingsError::Load)?;
        if let Some(path) = path {
            let contents = std::fs::read_to_string(path).map_err(SettingsError::Read)?;
            let contents = contents.strip_prefix('\u{feff}').unwrap_or(&contents);
            builder = builder.add_source(File::from_str(contents, FileFormat::Toml));
        }
        finish(
            builder.add_source(
                Environment::with_prefix("UNELOS").source(Some(
                    std::env::vars_os()
                        .filter_map(|(key, value)| {
                            let key = key.into_string().ok()?;
                            if key.eq_ignore_ascii_case("UNELOS_CONFIG_DIR")
                                || key.eq_ignore_ascii_case("UNELOS_CLAUDE_BIN")
                            {
                                return None;
                            }
                            Some((key, value.into_string().ok()?))
                        })
                        .collect(),
                )),
            ),
            log_level,
        )
    }
}

fn finish(
    mut builder: ConfigBuilder<DefaultState>,
    log_level: Option<&str>,
) -> Result<Settings, SettingsError> {
    if let Some(log_level) = log_level {
        builder = builder
            .set_override("log_level", log_level)
            .map_err(SettingsError::Load)?;
    }
    let settings: Settings = builder
        .build()
        .map_err(SettingsError::Load)?
        .try_deserialize()
        .map_err(SettingsError::Load)?;
    if !matches!(
        settings.log_level.as_str(),
        "off" | "error" | "warn" | "info" | "debug" | "trace"
    ) {
        return Err(SettingsError::InvalidLogLevel);
    }
    tracing::debug!("settings loaded and validated");
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config, Environment, File, FileFormat};
    use std::collections::HashMap;

    fn builder(
        toml: Option<&str>,
        level: Option<&str>,
    ) -> config::ConfigBuilder<config::builder::DefaultState> {
        let mut builder = Config::builder().set_default("log_level", "warn").unwrap();
        if let Some(toml) = toml {
            builder = builder.add_source(File::from_str(toml, FileFormat::Toml));
        }
        let env = level.map(|level| ("UNELOS_LOG_LEVEL".to_owned(), level.to_owned()));
        builder.add_source(Environment::with_prefix("UNELOS").source(Some(HashMap::from_iter(env))))
    }

    #[test]
    fn precedence_preserves_explicit_overrides() {
        assert_eq!(finish(builder(None, None), None).unwrap().log_level, "warn");
        assert_eq!(
            finish(builder(Some("log_level = 'error'"), None), None)
                .unwrap()
                .log_level,
            "error"
        );
        assert_eq!(
            finish(builder(Some("log_level = 'error'"), Some("info")), None)
                .unwrap()
                .log_level,
            "info"
        );
        assert_eq!(
            finish(
                builder(Some("log_level = 'error'"), Some("info")),
                Some("debug")
            )
            .unwrap()
            .log_level,
            "debug"
        );
    }

    #[test]
    fn invalid_settings_are_safe_to_describe() {
        for input in [
            "log_level = 'do-not-leak'",
            "token = 'do-not-leak'",
            "log_level = 'do-not-leak",
        ] {
            let error = finish(builder(Some(input), None), None).unwrap_err();
            assert!(!error.to_string().contains("do-not-leak"));
        }
    }

    #[test]
    fn all_supported_log_levels_validate() {
        for level in ["off", "error", "warn", "info", "debug", "trace"] {
            assert_eq!(
                finish(builder(None, None), Some(level)).unwrap().log_level,
                level
            );
        }
    }
}
