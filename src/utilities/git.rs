use std::path::Path;
use std::process::{Command, Output};

/// Git repository manager
pub struct GitRepo {
    repo_path: std::path::PathBuf,
}

impl GitRepo {
    /// Create a new GitRepo for an existing repository
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
        }
    }

    /// Clone a repository to the specified path
    ///
    /// # Arguments
    /// * `url` - The git repository URL
    /// * `dest_path` - Destination path for the cloned repository
    /// * `branch` - Optional branch to clone
    pub fn clone<P: AsRef<Path>>(
        url: &str,
        dest_path: P,
        branch: Option<&str>,
    ) -> Result<Self, String> {
        let dest_path = dest_path.as_ref();

        // Check if directory already exists
        if dest_path.exists() {
            return Ok(Self::new(dest_path));
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }

        let mut cmd = Command::new("git");
        cmd.arg("clone");

        if let Some(branch) = branch {
            cmd.arg("--branch").arg(branch);
        }

        cmd.arg(url).arg(dest_path);

        let output = cmd.output().map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to clone repository: {}", stderr));
        }

        Ok(Self::new(dest_path))
    }

    /// Get the current commit hash
    pub fn get_commit_hash(&self) -> Result<String, String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to get commit hash: {}", stderr));
        }

        let hash = String::from_utf8_lossy(&output.stdout);
        Ok(hash.trim().to_string())
    }

    /// Get a short version of the commit hash (first 7 characters)
    pub fn get_short_commit_hash(&self) -> Result<String, String> {
        let hash = self.get_commit_hash()?;
        Ok(hash.chars().take(7).collect())
    }

    /// Get the current branch name
    pub fn get_current_branch(&self) -> Result<String, String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to get branch name: {}", stderr));
        }

        let branch = String::from_utf8_lossy(&output.stdout);
        Ok(branch.trim().to_string())
    }

    /// Checkout a branch or tag
    pub fn checkout(&self, ref_name: &str) -> Result<(), String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("checkout")
            .arg(ref_name)
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to checkout {}: {}", ref_name, stderr));
        }

        Ok(())
    }

    /// Pull latest changes from remote
    pub fn pull(&self) -> Result<Output, String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("pull")
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to pull: {}", stderr));
        }

        Ok(output)
    }

    /// Fetch from remote without merging
    pub fn fetch(&self) -> Result<Output, String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("fetch")
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to fetch: {}", stderr));
        }

        Ok(output)
    }

    /// Check if the repository has uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool, String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .arg("status")
            .arg("--porcelain")
            .output()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to check status: {}", stderr));
        }

        // If output is not empty, there are uncommitted changes
        Ok(!output.stdout.is_empty())
    }

    /// Get the repository path
    pub fn path(&self) -> &Path {
        &self.repo_path
    }
}

/// Check if git is available on the system
pub fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok()
}

/// Get the git version
pub fn get_git_version() -> Result<String, String> {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if !output.status.success() {
        return Err("Failed to get git version".to_string());
    }

    let version = String::from_utf8_lossy(&output.stdout);
    Ok(version.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_git_available() {
        // This should pass on most systems
        assert!(is_git_available());
    }

    #[test]
    fn test_get_git_version() {
        let result = get_git_version();
        assert!(result.is_ok());
        let version = result.unwrap();
        assert!(version.contains("git version"));
    }

    #[test]
    #[ignore] // This requires network access
    fn test_clone_repository() {
        let temp_dir = TempDir::new().unwrap();
        let dest_path = temp_dir.path().join("test-repo");

        // Clone a small public repository
        let result = GitRepo::clone(
            "https://github.com/octocat/Hello-World.git",
            &dest_path,
            None,
        );

        assert!(result.is_ok());
        assert!(dest_path.exists());
        assert!(dest_path.join(".git").exists());
    }

    #[test]
    fn test_git_repo_operations() {
        // Create a test git repository
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_path).unwrap();

        // Initialize git repo
        Command::new("git")
            .current_dir(&repo_path)
            .arg("init")
            .output()
            .unwrap();

        // Configure git user for testing
        Command::new("git")
            .current_dir(&repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .output()
            .unwrap();

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create a file and commit it
        fs::write(repo_path.join("test.txt"), "test content").unwrap();
        Command::new("git")
            .current_dir(&repo_path)
            .args(&["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(&repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .output()
            .unwrap();

        // Test GitRepo operations
        let repo = GitRepo::new(&repo_path);

        // Test get_commit_hash
        let hash = repo.get_commit_hash();
        assert!(hash.is_ok());
        let hash = hash.unwrap();
        assert_eq!(hash.len(), 40); // Full SHA-1 hash

        // Test get_short_commit_hash
        let short_hash = repo.get_short_commit_hash();
        assert!(short_hash.is_ok());
        let short_hash = short_hash.unwrap();
        assert_eq!(short_hash.len(), 7);

        // Test get_current_branch
        let branch = repo.get_current_branch();
        assert!(branch.is_ok());
        // Branch name varies by git version (master or main)
        let branch = branch.unwrap();
        assert!(branch == "master" || branch == "main");

        // Test has_uncommitted_changes (should be false after commit)
        let has_changes = repo.has_uncommitted_changes();
        assert!(has_changes.is_ok());
        assert!(!has_changes.unwrap());

        // Make a change
        fs::write(repo_path.join("test.txt"), "modified content").unwrap();

        // Now should have uncommitted changes
        let has_changes = repo.has_uncommitted_changes();
        assert!(has_changes.is_ok());
        assert!(has_changes.unwrap());
    }
}

