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
    pub main: RepositoryInfo,
}

#[derive(Debug, Deserialize)]
pub struct RepositoryInfo {
    pub url: String,
    pub branch: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config(dir: &TempDir) -> PathBuf {
        let config_path = dir.path().join("test_config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"

[repositories.main]
url = "http://github.com/test/repo.git"
branch = "main"
"#
        )
        .unwrap();
        config_path
    }

    #[test]
    fn test_load_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);

        let config = Config::load_from_path(&config_path).unwrap();

        assert_eq!(config.local_storage, "~/.local/share/sourcery");
        assert_eq!(config.install.user_path, "~/.local/bin");
        assert_eq!(config.install.system_path, "/usr/local/bin");
        assert_eq!(config.build.default_environment, "container");
        assert_eq!(config.repositories.main.url, "http://github.com/test/repo.git");
        assert_eq!(config.repositories.main.branch, "main");
    }

    #[test]
    fn test_load_from_path_invalid_file() {
        let result = Config::load_from_path(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_path_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(file, "this is not valid toml [[[").unwrap();

        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_locations() {
        let locations = Config::config_locations();
        assert_eq!(locations.len(), 3);
        assert_eq!(locations[0], "config/sourcery.toml");
        assert_eq!(locations[1], "~/.config/sourcery/sourcery.toml");
        assert_eq!(locations[2], "/etc/sourcery/sourcery.toml");
    }

    #[test]
    fn test_local_storage_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);
        let config = Config::load_from_path(&config_path).unwrap();

        let path = config.local_storage_path();
        // Should expand tilde
        assert!(path.to_string_lossy().contains(".local/share/sourcery"));
        assert!(!path.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn test_user_install_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);
        let config = Config::load_from_path(&config_path).unwrap();

        let path = config.user_install_path();
        assert!(path.to_string_lossy().contains(".local/bin"));
        assert!(!path.to_string_lossy().starts_with('~'));
    }

    #[test]
    fn test_system_install_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);
        let config = Config::load_from_path(&config_path).unwrap();

        let path = config.system_install_path();
        assert_eq!(path, PathBuf::from("/usr/local/bin"));
    }
}

