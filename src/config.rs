use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::{expand_tilde, find_file_in_locations};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub local_storage: String,
    pub install: InstallConfig,
    pub build: BuildConfig,
    pub repositories: RepositoriesConfig,
}

#[derive(Debug, Deserialize)]
pub struct InstallConfig {
    pub user_path: String,
    pub system_path: String,
}

#[derive(Debug, Deserialize)]
pub struct BuildConfig {
    pub default_environment: String,
}

#[derive(Debug, Deserialize)]
pub struct RepositoriesConfig {
    pub main: String,
}

impl Config {
    /// Load configuration from the standard hierarchy of locations
    pub fn load() -> Result<(Self, PathBuf), Box<dyn std::error::Error>> {
        let config_locations = vec![
            "config/sourcery.toml",
            "~/.config/sourcery/sourcery.toml",
            "/etc/sourcery/sourcery.toml",
        ];

        let config_path = find_file_in_locations(&config_locations)
            .ok_or("No config file found")?;

        let config = Self::load_from_path(&config_path)?;
        Ok((config, config_path))
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the config locations that are checked in order
    pub fn config_locations() -> Vec<&'static str> {
        vec![
            "config/sourcery.toml",
            "~/.config/sourcery/sourcery.toml",
            "/etc/sourcery/sourcery.toml",
        ]
    }

    /// Get the expanded local storage path
    pub fn local_storage_path(&self) -> PathBuf {
        expand_tilde(&self.local_storage)
    }

    /// Get the expanded user install path
    pub fn user_install_path(&self) -> PathBuf {
        expand_tilde(&self.install.user_path)
    }

    /// Get the expanded system install path
    pub fn system_install_path(&self) -> PathBuf {
        expand_tilde(&self.install.system_path)
    }
}

