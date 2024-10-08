use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Error loading configuration file: {0}")]
    LoadingConfigFile(String),
    #[error("Error parsing configuration file: {0}")]
    ParsingConfigFile(String),
    #[error("Error discovering configuration file: {0}")]
    ConfigDiscovery(String),
}

/// Load the configuration from a file.
pub fn load_config<T>(path: &Path) -> Result<T, ConfigError>
where
    T: DeserializeOwned,
{
    let mut file = File::open(path).map_err(|e| ConfigError::LoadingConfigFile(e.to_string()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| ConfigError::LoadingConfigFile(e.to_string()))?;

    let config: T =
        toml::from_str(&contents).map_err(|e| ConfigError::ParsingConfigFile(e.to_string()))?;

    Ok(config)
}

/// Discover the location of the wgpull configuration file.
///
/// Returns /etc/wgpull.conf if it exists, or ./wgpull.conf if it
/// exists in the current working directory.
pub fn discover_config_path(filename: &'static str) -> Result<PathBuf, ConfigError> {
    let paths = vec!["/etc/wgpull", "."];

    for path in paths {
        let candidate = PathBuf::from(path).join(filename);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(ConfigError::ConfigDiscovery(format!(
        "Could not find {}",
        filename
    )))
}
