use std::fs::File;
use std::io::Read;
use thiserror::Error;
use serde::de::DeserializeOwned;

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
pub fn load_config<T>(path: &str) -> Result<T, ConfigError>
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
pub fn discover_config_path() -> Result<&'static str, ConfigError> {
    let paths = vec!["/etc/wgpull/wgpull.conf", "./wgpull.conf"];

    for path in paths {
        if std::path::Path::new(path).exists() {
            return Ok(path);
        }
    }

    Err(ConfigError::ConfigDiscovery(
        "Could not find wgpull.conf".to_string(),
    ))
}
