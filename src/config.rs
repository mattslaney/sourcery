use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::{expand_tilde, find_file_in_locations};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    pub local_storage: String,
    pub install: InstallConfig,
    pub build: BuildConfig,
    pub repositories: HashMap<String, RepositoryInfo>,
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

fn default_log_level() -> String {
    "info".to_string()
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
    pub url: Option<String>,
    pub branch: Option<String>,
    pub path: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

impl RepositoryInfo {
    /// Returns true if this is a path-based repository
    pub fn is_path_based(&self) -> bool {
        self.path.is_some()
    }

    /// Returns true if this is a URL-based repository
    pub fn is_url_based(&self) -> bool {
        self.url.is_some()
    }

    /// Validates the repository configuration
    /// A repository must have either (url + branch) OR path, but not both
    pub fn validate(&self) -> Result<(), String> {
        let has_url = self.url.is_some();
        let has_path = self.path.is_some();
        let has_branch = self.branch.is_some();

        if !has_url && !has_path {
            return Err("Repository must have either 'url' or 'path' specified".to_string());
        }

        if has_url && has_path {
            return Err("Repository cannot have both 'url' and 'path' specified".to_string());
        }

        if has_url && !has_branch {
            return Err("URL-based repository must have 'branch' specified".to_string());
        }

        if has_path && has_branch {
            return Err("Path-based repository should not have 'branch' specified".to_string());
        }

        Ok(())
    }

    /// Get the expanded path for path-based repositories
    /// If the path is relative, it will be resolved relative to the config file's directory
    /// If config_path is provided and the path is relative, resolve relative to config dir
    pub fn expanded_path(&self, config_path: Option<&Path>) -> Option<PathBuf> {
        self.path.as_ref().map(|p| {
            let expanded = expand_tilde(p);
            
            // If the path is relative and we have a config path, make it relative to config dir
            if expanded.is_relative() {
                if let Some(cfg_path) = config_path {
                    // Resolve the config path (handling symlinks)
                    let real_config_path = fs::canonicalize(cfg_path)
                        .unwrap_or_else(|_| cfg_path.to_path_buf());
                    
                    // Get the config directory
                    if let Some(config_dir) = real_config_path.parent() {
                        return config_dir.join(expanded);
                    }
                }
            }
            
            expanded
        })
    }
}

fn default_priority() -> u32 {
    100
}

impl Config {
    /// Load configuration from the standard hierarchy of locations
    pub fn load() -> Result<(Self, PathBuf), Box<dyn std::error::Error>> {
        // Check if SOURCERY_CONFIG_PATH environment variable is set (for testing)
        if let Ok(test_config_path) = std::env::var("SOURCERY_CONFIG_PATH") {
            let path = PathBuf::from(&test_config_path);
            if path.exists() {
                let config = Self::load_from_path(&path)?;
                return Ok((config, path));
            }
        }

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
        let mut config: Config = toml::from_str(&content)?;
        
        // Store the config path (resolved, handling symlinks)
        config.config_path = Some(path.to_path_buf());
        
        // Validate all repositories
        for (name, repo_info) in &config.repositories {
            if let Err(e) = repo_info.validate() {
                return Err(format!("Invalid repository '{}': {}", name, e).into());
            }
        }
        
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

    /// Get the expanded path for a path-based repository
    /// Handles relative paths (relative to config file) and tilde expansion
    pub fn get_repository_path(&self, repo_info: &RepositoryInfo) -> Option<PathBuf> {
        repo_info.expanded_path(self.config_path.as_deref())
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
log_level = "info"
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

        assert_eq!(config.log_level, "info");
        assert_eq!(config.local_storage, "~/.local/share/sourcery");
        assert_eq!(config.install.user_path, "~/.local/bin");
        assert_eq!(config.install.system_path, "/usr/local/bin");
        assert_eq!(config.build.default_environment, "container");
        assert_eq!(
            config.repositories.get("main").unwrap().url,
            Some("http://github.com/test/repo.git".to_string())
        );
        assert_eq!(
            config.repositories.get("main").unwrap().branch,
            Some("main".to_string())
        );
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
log_level = "info"
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
        assert_eq!(
            config.repositories.get("main").unwrap().branch,
            Some("main".to_string())
        );
        assert_eq!(config.repositories.get("main").unwrap().priority, 10);

        // Verify wip repository
        assert!(config.repositories.contains_key("wip"));
        assert_eq!(
            config.repositories.get("wip").unwrap().branch,
            Some("wip".to_string())
        );
        assert_eq!(config.repositories.get("wip").unwrap().priority, 20);

        // Verify dev repository
        assert!(config.repositories.contains_key("dev"));
        assert_eq!(
            config.repositories.get("dev").unwrap().url,
            Some("http://github.com/test/dev-repo.git".to_string())
        );
        assert_eq!(
            config.repositories.get("dev").unwrap().branch,
            Some("develop".to_string())
        );
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
log_level = "info"
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
log_level = "info"
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

    #[test]
    fn test_default_log_level() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("no_log_level.toml");
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

        // Log level should default to "info"
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_path_based_repository() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("path_repo_config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
log_level = "info"
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"

[repositories.local]
path = "/path/to/local/repo"
priority = 5
"#
        )
        .unwrap();

        let config = Config::load_from_path(&config_path).unwrap();

        assert_eq!(config.repositories.len(), 1);
        let local_repo = config.repositories.get("local").unwrap();
        assert_eq!(local_repo.path, Some("/path/to/local/repo".to_string()));
        assert_eq!(local_repo.url, None);
        assert_eq!(local_repo.branch, None);
        assert_eq!(local_repo.priority, 5);
        assert!(local_repo.is_path_based());
        assert!(!local_repo.is_url_based());
    }

    #[test]
    fn test_mixed_url_and_path_repositories() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("mixed_repos.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
log_level = "info"
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"

[repositories.remote]
url = "http://github.com/test/repo.git"
branch = "main"
priority = 10

[repositories.local]
path = "/path/to/local/repo"
priority = 5
"#
        )
        .unwrap();

        let config = Config::load_from_path(&config_path).unwrap();

        assert_eq!(config.repositories.len(), 2);
        
        let remote_repo = config.repositories.get("remote").unwrap();
        assert!(remote_repo.is_url_based());
        assert!(!remote_repo.is_path_based());
        
        let local_repo = config.repositories.get("local").unwrap();
        assert!(local_repo.is_path_based());
        assert!(!local_repo.is_url_based());

        // Check priority ordering
        let repos = config.repositories_by_priority();
        assert_eq!(repos[0].0, "local"); // priority 5
        assert_eq!(repos[1].0, "remote"); // priority 10
    }

    #[test]
    fn test_repository_validation_no_url_or_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid_repo.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
log_level = "info"
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"

[repositories.invalid]
priority = 10
"#
        )
        .unwrap();

        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must have either 'url' or 'path'"));
    }

    #[test]
    fn test_repository_validation_both_url_and_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("both_url_and_path.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
log_level = "info"
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"

[repositories.invalid]
url = "http://github.com/test/repo.git"
path = "/path/to/local/repo"
branch = "main"
"#
        )
        .unwrap();

        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot have both 'url' and 'path'"));
    }

    #[test]
    fn test_repository_validation_url_without_branch() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("url_no_branch.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
log_level = "info"
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"

[repositories.invalid]
url = "http://github.com/test/repo.git"
"#
        )
        .unwrap();

        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must have 'branch' specified"));
    }

    #[test]
    fn test_repository_validation_path_with_branch() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("path_with_branch.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            r#"
log_level = "info"
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"

[repositories.invalid]
path = "/path/to/local/repo"
branch = "main"
"#
        )
        .unwrap();

        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("should not have 'branch' specified"));
    }

    #[test]
    fn test_repository_expanded_path_absolute() {
        let repo_info = RepositoryInfo {
            url: None,
            branch: None,
            path: Some("~/local/repo".to_string()),
            priority: 10,
        };

        let expanded = repo_info.expanded_path(None).unwrap();
        // Should expand tilde
        assert!(!expanded.to_string_lossy().starts_with('~'));
        assert!(expanded.to_string_lossy().contains("local/repo"));
    }

    #[test]
    fn test_repository_expanded_path_relative_to_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config").join("sourcery.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::File::create(&config_path).unwrap();

        let repo_info = RepositoryInfo {
            url: None,
            branch: None,
            path: Some("../repos/local-repo".to_string()),
            priority: 10,
        };

        let expanded = repo_info.expanded_path(Some(&config_path)).unwrap();
        
        // Should resolve relative to config directory
        // temp_dir/config/sourcery.toml + ../repos/local-repo = temp_dir/repos/local-repo
        assert!(expanded.to_string_lossy().ends_with("repos/local-repo"));
        assert!(expanded.to_string_lossy().contains(temp_dir.path().to_str().unwrap()));
    }

    #[test]
    fn test_repository_expanded_path_absolute_ignores_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config").join("sourcery.toml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::File::create(&config_path).unwrap();

        let repo_info = RepositoryInfo {
            url: None,
            branch: None,
            path: Some("/absolute/path/to/repo".to_string()),
            priority: 10,
        };

        let expanded = repo_info.expanded_path(Some(&config_path)).unwrap();
        
        // Absolute path should not be modified by config path
        assert_eq!(expanded, PathBuf::from("/absolute/path/to/repo"));
    }
}
