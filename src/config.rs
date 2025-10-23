use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::{expand_tilde, find_file_in_locations};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub local_storage: String,
    pub install: InstallConfig,
    pub build: BuildConfig,
    pub repositories: HashMap<String, RepositoryInfo>,
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

#[derive(Debug, Deserialize, Clone)]
pub struct RepositoryInfo {
    pub url: String,
    pub branch: String,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 {
    100
}

impl Config {
    /// Load configuration from the standard hierarchy of locations
    pub fn load() -> Result<(Self, PathBuf), Box<dyn std::error::Error>> {
        let config_locations = vec![
            "config/sourcery.toml",
            "~/.config/sourcery/sourcery.toml",
            "/etc/sourcery/sourcery.toml",
        ];

        let config_path =
            find_file_in_locations(&config_locations).ok_or("No config file found")?;

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

    /// Get repositories sorted by priority (lowest priority number first)
    pub fn repositories_by_priority(&self) -> Vec<(String, RepositoryInfo)> {
        let mut repos: Vec<_> = self
            .repositories
            .iter()
            .map(|(name, info)| (name.clone(), info.clone()))
            .collect();
        repos.sort_by_key(|(_, info)| info.priority);
        repos
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
priority = 10
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
        assert_eq!(
            config.repositories.get("main").unwrap().url,
            "http://github.com/test/repo.git"
        );
        assert_eq!(config.repositories.get("main").unwrap().branch, "main");
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

    #[test]
    fn test_multiple_repositories() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("multi_repo_config.toml");
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
priority = 10

[repositories.wip]
url = "http://github.com/test/repo.git"
branch = "wip"
priority = 20

[repositories.dev]
url = "http://github.com/test/dev-repo.git"
branch = "develop"
priority = 15
"#
        )
        .unwrap();

        let config = Config::load_from_path(&config_path).unwrap();

        // Verify we have 3 repositories
        assert_eq!(config.repositories.len(), 3);

        // Verify main repository
        assert!(config.repositories.contains_key("main"));
        assert_eq!(config.repositories.get("main").unwrap().branch, "main");
        assert_eq!(config.repositories.get("main").unwrap().priority, 10);

        // Verify wip repository
        assert!(config.repositories.contains_key("wip"));
        assert_eq!(config.repositories.get("wip").unwrap().branch, "wip");
        assert_eq!(config.repositories.get("wip").unwrap().priority, 20);

        // Verify dev repository
        assert!(config.repositories.contains_key("dev"));
        assert_eq!(
            config.repositories.get("dev").unwrap().url,
            "http://github.com/test/dev-repo.git"
        );
        assert_eq!(config.repositories.get("dev").unwrap().branch, "develop");
        assert_eq!(config.repositories.get("dev").unwrap().priority, 15);
    }

    #[test]
    fn test_single_repository() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);
        let config = Config::load_from_path(&config_path).unwrap();

        // Verify we have 1 repository
        assert_eq!(config.repositories.len(), 1);
        assert!(config.repositories.contains_key("main"));
    }

    #[test]
    fn test_repositories_by_priority() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("priority_test.toml");
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
priority = 30

[repositories.wip]
url = "http://github.com/test/repo.git"
branch = "wip"
priority = 10

[repositories.dev]
url = "http://github.com/test/dev-repo.git"
branch = "develop"
priority = 20
"#
        )
        .unwrap();

        let config = Config::load_from_path(&config_path).unwrap();
        let repos = config.repositories_by_priority();

        // Should be sorted by priority: wip (10), dev (20), main (30)
        assert_eq!(repos.len(), 3);
        assert_eq!(repos[0].0, "wip");
        assert_eq!(repos[0].1.priority, 10);
        assert_eq!(repos[1].0, "dev");
        assert_eq!(repos[1].1.priority, 20);
        assert_eq!(repos[2].0, "main");
        assert_eq!(repos[2].1.priority, 30);
    }

    #[test]
    fn test_default_priority() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("default_priority_test.toml");
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

        let config = Config::load_from_path(&config_path).unwrap();

        // Priority should default to 100
        assert_eq!(config.repositories.get("main").unwrap().priority, 100);
    }
}
