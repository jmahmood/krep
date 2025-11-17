//! Configuration file support for Krep.
//!
//! Configuration is loaded from `$XDG_CONFIG_HOME/krep/config.toml`.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Application configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub data: DataConfig,

    #[serde(default)]
    pub equipment: EquipmentConfig,

    #[serde(default)]
    pub progression: ProgressionConfig,

    #[serde(default)]
    pub mobility: MobilityConfig,
}

/// Data storage configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
        }
    }
}

/// Equipment availability configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquipmentConfig {
    #[serde(default = "default_equipment")]
    pub available: Vec<String>,
}

impl Default for EquipmentConfig {
    fn default() -> Self {
        Self {
            available: default_equipment(),
        }
    }
}

/// Progression parameters configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProgressionConfig {
    #[serde(default = "default_burpee_rep_ceiling")]
    pub burpee_rep_ceiling: i32,

    #[serde(default = "default_kb_swing_max_reps")]
    pub kb_swing_max_reps: i32,
}

impl Default for ProgressionConfig {
    fn default() -> Self {
        Self {
            burpee_rep_ceiling: default_burpee_rep_ceiling(),
            kb_swing_max_reps: default_kb_swing_max_reps(),
        }
    }
}

/// Custom mobility drill definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomMobilityDrill {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

/// Mobility drills configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MobilityConfig {
    #[serde(default)]
    pub custom: Vec<CustomMobilityDrill>,
}

// Default value functions
fn default_data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| {
        let home = std::env::var("HOME")
            .expect("HOME environment variable not set");
        PathBuf::from(home).join(".local/share")
    });
    base.join("krep")
}

fn default_equipment() -> Vec<String> {
    vec![
        "kettlebell".into(),
        "pullup_bar".into(),
        "bands".into(),
    ]
}

fn default_burpee_rep_ceiling() -> i32 {
    10
}

fn default_kb_swing_max_reps() -> i32 {
    15
}

impl Config {
    /// Load configuration from the standard config path
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path();
        if config_path.exists() {
            Self::load_from(&config_path)
        } else {
            tracing::info!(
                "No config file found at {:?}, using defaults",
                config_path
            );
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        tracing::info!("Loaded config from {:?}", path);
        Ok(config)
    }

    /// Get the default config file path
    pub fn default_config_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .expect("HOME environment variable not set");
            PathBuf::from(home).join(".config")
        });
        base.join("krep").join("config.toml")
    }

    /// Save the current configuration to the default path
    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_config_path();
        self.save_to(&config_path)
    }

    /// Save the current configuration to a specific path
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, contents)?;
        tracing::info!("Saved config to {:?}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.equipment.available.is_empty());
        assert_eq!(config.progression.burpee_rep_ceiling, 10);
        assert_eq!(config.progression.kb_swing_max_reps, 15);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(
            config.progression.burpee_rep_ceiling,
            parsed.progression.burpee_rep_ceiling
        );
        assert_eq!(
            config.equipment.available,
            parsed.equipment.available
        );
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
[progression]
burpee_rep_ceiling = 12
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.progression.burpee_rep_ceiling, 12);
        assert_eq!(config.progression.kb_swing_max_reps, 15); // default
    }
}
