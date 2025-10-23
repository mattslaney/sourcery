use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents the build environment for a package
#[derive(Debug, Clone)]
pub struct BuildEnvironment {
    /// Package name
    pub package_name: String,
    /// Commit hash (for versioning artifacts)
    pub commit_hash: String,
    /// Branch name
    pub branch: Option<String>,
    /// Base directory for all sourcery data (~/.local/share/sourcery)
    pub base_dir: PathBuf,
    /// Directory where source code is cloned
    pub source_dir: PathBuf,
    /// Directory where artifacts are stored
    pub artifacts_dir: PathBuf,
    /// Install prefix (/usr/local or ~/.local)
    pub install_prefix: PathBuf,
}

impl BuildEnvironment {
    /// Create a new BuildEnvironment
    pub fn new(
        package_name: String,
        commit_hash: String,
        branch: Option<String>,
        base_dir: PathBuf,
        install_prefix: PathBuf,
    ) -> Self {
        let source_dir = base_dir.join("source").join(&package_name);
        let artifacts_dir = base_dir
            .join("artifacts")
            .join(&package_name)
            .join(&commit_hash);

        Self {
            package_name,
            commit_hash,
            branch,
            base_dir,
            source_dir,
            artifacts_dir,
            install_prefix,
        }
    }

    /// Get the environment variables for this build environment
    pub fn get_env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Source locations
        env.insert(
            "SCRY_SRC".to_string(),
            self.source_dir.to_string_lossy().to_string(),
        );
        env.insert(
            "SCRY_BUILD".to_string(),
            self.source_dir.to_string_lossy().to_string(),
        );

        // Output locations
        env.insert("SCRY_COMMIT".to_string(), self.commit_hash.clone());
        env.insert(
            "SCRY_ARTIFACTS".to_string(),
            self.artifacts_dir.to_string_lossy().to_string(),
        );
        env.insert(
            "SCRY_OUT".to_string(),
            self.artifacts_dir.to_string_lossy().to_string(),
        );

        // Install locations
        env.insert(
            "SCRY_PREFIX".to_string(),
            self.install_prefix.to_string_lossy().to_string(),
        );
        env.insert(
            "SCRY_BINDIR".to_string(),
            self.install_prefix
                .join("bin")
                .to_string_lossy()
                .to_string(),
        );
        env.insert(
            "SCRY_LIBDIR".to_string(),
            self.install_prefix
                .join("lib")
                .to_string_lossy()
                .to_string(),
        );
        env.insert(
            "SCRY_DATADIR".to_string(),
            self.install_prefix
                .join("share")
                .to_string_lossy()
                .to_string(),
        );

        // Package metadata
        env.insert("SCRY_PACKAGE".to_string(), self.package_name.clone());
        if let Some(branch) = &self.branch {
            env.insert("SCRY_BRANCH".to_string(), branch.clone());
        }

        env
    }

    /// Get environment variables as a Vec of (key, value) tuples for command execution
    pub fn get_env_vars_vec(&self) -> Vec<(String, String)> {
        self.get_env_vars().into_iter().collect()
    }

    /// Create necessary directories for the build environment
    pub fn setup_directories(&self) -> Result<(), std::io::Error> {
        // Create source directory
        fs::create_dir_all(&self.source_dir)?;

        // Create artifacts directory
        fs::create_dir_all(&self.artifacts_dir)?;

        // Create logs directory
        let logs_dir = self.base_dir.join("logs").join("packages").join(&self.package_name);
        fs::create_dir_all(&logs_dir)?;

        Ok(())
    }

    /// Get the log file path for this build
    pub fn get_log_file_path(&self) -> PathBuf {
        self.base_dir
            .join("logs")
            .join("packages")
            .join(&self.package_name)
            .join(format!("{}.log", self.commit_hash))
    }

    /// Check if artifacts exist for this commit
    pub fn artifacts_exist(&self) -> bool {
        self.artifacts_dir.exists() && self.artifacts_dir.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false)
    }

    /// Get the repository directory for a specific repository name
    pub fn get_repository_dir(&self, repo_name: &str) -> PathBuf {
        self.base_dir.join("repositories").join(repo_name)
    }
}

/// Helper to convert environment variables to container-friendly format
pub fn format_env_for_container(env_vars: &HashMap<String, String>) -> Vec<(&str, &str)> {
    env_vars
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect()
}

/// Helper to get volume mounts for a container build
/// Returns (host_path, container_path) tuples
pub fn get_build_volume_mounts(build_env: &BuildEnvironment) -> Vec<(PathBuf, PathBuf)> {
    vec![
        // Mount source directory to /usr/src/package
        (build_env.source_dir.clone(), PathBuf::from("/usr/src/package")),
        // Mount artifacts directory to /artifacts
        (build_env.artifacts_dir.clone(), PathBuf::from("/artifacts")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_build_environment_creation() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let install_prefix = PathBuf::from("/usr/local");

        let env = BuildEnvironment::new(
            "neovim".to_string(),
            "abc123".to_string(),
            Some("stable".to_string()),
            base_dir.clone(),
            install_prefix.clone(),
        );

        assert_eq!(env.package_name, "neovim");
        assert_eq!(env.commit_hash, "abc123");
        assert_eq!(env.branch, Some("stable".to_string()));
        assert_eq!(env.source_dir, base_dir.join("source").join("neovim"));
        assert_eq!(
            env.artifacts_dir,
            base_dir.join("artifacts").join("neovim").join("abc123")
        );
        assert_eq!(env.install_prefix, install_prefix);
    }

    #[test]
    fn test_get_env_vars() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let install_prefix = PathBuf::from("/usr/local");

        let env = BuildEnvironment::new(
            "neovim".to_string(),
            "abc123".to_string(),
            Some("stable".to_string()),
            base_dir.clone(),
            install_prefix.clone(),
        );

        let vars = env.get_env_vars();

        assert_eq!(
            vars.get("SCRY_SRC").unwrap(),
            &base_dir.join("source").join("neovim").to_string_lossy()
        );
        assert_eq!(vars.get("SCRY_COMMIT").unwrap(), "abc123");
        assert_eq!(
            vars.get("SCRY_ARTIFACTS").unwrap(),
            &base_dir
                .join("artifacts")
                .join("neovim")
                .join("abc123")
                .to_string_lossy()
        );
        assert_eq!(vars.get("SCRY_PACKAGE").unwrap(), "neovim");
        assert_eq!(vars.get("SCRY_BRANCH").unwrap(), "stable");
        assert_eq!(
            vars.get("SCRY_PREFIX").unwrap(),
            &install_prefix.to_string_lossy()
        );
        assert_eq!(
            vars.get("SCRY_BINDIR").unwrap(),
            &install_prefix.join("bin").to_string_lossy()
        );
    }

    #[test]
    fn test_setup_directories() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let install_prefix = PathBuf::from("/usr/local");

        let env = BuildEnvironment::new(
            "neovim".to_string(),
            "abc123".to_string(),
            None,
            base_dir.clone(),
            install_prefix,
        );

        let result = env.setup_directories();
        assert!(result.is_ok());

        // Verify directories were created
        assert!(env.source_dir.exists());
        assert!(env.artifacts_dir.exists());
        assert!(base_dir
            .join("logs")
            .join("packages")
            .join("neovim")
            .exists());
    }

    #[test]
    fn test_get_log_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let install_prefix = PathBuf::from("/usr/local");

        let env = BuildEnvironment::new(
            "neovim".to_string(),
            "abc123".to_string(),
            None,
            base_dir.clone(),
            install_prefix,
        );

        let log_path = env.get_log_file_path();
        assert_eq!(
            log_path,
            base_dir
                .join("logs")
                .join("packages")
                .join("neovim")
                .join("abc123.log")
        );
    }

    #[test]
    fn test_artifacts_exist() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let install_prefix = PathBuf::from("/usr/local");

        let env = BuildEnvironment::new(
            "neovim".to_string(),
            "abc123".to_string(),
            None,
            base_dir.clone(),
            install_prefix,
        );

        // Initially should not exist
        assert!(!env.artifacts_exist());

        // Create directories
        env.setup_directories().unwrap();

        // Still empty, so should return false
        assert!(!env.artifacts_exist());

        // Create a file in artifacts
        fs::write(env.artifacts_dir.join("test.txt"), "test").unwrap();

        // Now should exist
        assert!(env.artifacts_exist());
    }

    #[test]
    fn test_get_repository_dir() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let install_prefix = PathBuf::from("/usr/local");

        let env = BuildEnvironment::new(
            "neovim".to_string(),
            "abc123".to_string(),
            None,
            base_dir.clone(),
            install_prefix,
        );

        let repo_dir = env.get_repository_dir("main");
        assert_eq!(repo_dir, base_dir.join("repositories").join("main"));
    }

    #[test]
    fn test_get_build_volume_mounts() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let install_prefix = PathBuf::from("/usr/local");

        let env = BuildEnvironment::new(
            "neovim".to_string(),
            "abc123".to_string(),
            None,
            base_dir.clone(),
            install_prefix,
        );

        let mounts = get_build_volume_mounts(&env);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].0, env.source_dir);
        assert_eq!(mounts[0].1, PathBuf::from("/usr/src/package"));
        assert_eq!(mounts[1].0, env.artifacts_dir);
        assert_eq!(mounts[1].1, PathBuf::from("/artifacts"));
    }
}

