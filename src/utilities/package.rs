use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Represents a stage in the package lifecycle (build, install, update, uninstall, purge)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageStage {
    /// Dependencies required for this stage (installed via system package manager)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,

    /// Command to execute for this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Artifacts produced by this stage (typically for build stage)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<String>>,

    /// Prerequisites stages that must be executed before this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<Vec<String>>,

    /// Whether this stage requires elevated privileges
    /// Note: Handles both "privileged" and "priviledged" (typo in some files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privileged: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priviledged: Option<bool>,

    /// Installation scope restrictions (for install stage)
    /// Valid values: "system", "user"
    /// If empty or not provided, both scopes are allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<String>>,
}

impl PackageStage {
    /// Returns whether this stage requires elevated privileges
    /// Checks both correct and misspelled field names
    pub fn is_privileged(&self) -> bool {
        self.privileged.unwrap_or(false) || self.priviledged.unwrap_or(false)
    }

    /// Check if a given installation scope is allowed
    /// Returns true if scope is allowed, false otherwise
    /// If no scope restriction is defined (None or empty), all scopes are allowed
    /// Only "system" and "user" are recognized as valid scopes; others are ignored
    pub fn is_scope_allowed(&self, scope: &str) -> bool {
        match &self.scope {
            None => true, // No restriction, allow all scopes
            Some(scopes) => {
                if scopes.is_empty() {
                    true // Empty array means allow all scopes
                } else {
                    // Filter to only valid scopes (system/user), then check if requested scope matches
                    let valid_scopes: Vec<String> = scopes
                        .iter()
                        .map(|s| s.to_lowercase())
                        .filter(|s| s == "system" || s == "user")
                        .collect();
                    
                    // If no valid scopes remain after filtering, allow all
                    if valid_scopes.is_empty() {
                        true
                    } else {
                        valid_scopes.contains(&scope.to_lowercase())
                    }
                }
            }
        }
    }

    /// Merge another stage into this one, with the other stage taking precedence
    pub fn merge(&mut self, other: &PackageStage) {
        if other.dependencies.is_some() {
            self.dependencies = other.dependencies.clone();
        }
        if other.command.is_some() {
            self.command = other.command.clone();
        }
        if other.artifacts.is_some() {
            self.artifacts = other.artifacts.clone();
        }
        if other.prerequisites.is_some() {
            self.prerequisites = other.prerequisites.clone();
        }
        if other.privileged.is_some() {
            self.privileged = other.privileged;
        }
        if other.priviledged.is_some() {
            self.priviledged = other.priviledged;
        }
        if other.scope.is_some() {
            self.scope = other.scope.clone();
        }
    }
}

/// Represents a package definition loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package name
    #[serde(default)]
    pub name: String,

    /// Package description
    #[serde(default)]
    pub desc: String,

    /// Repository URLs for source code
    #[serde(default)]
    pub repo: Vec<String>,

    /// Default branch to use
    #[serde(default)]
    pub branch: String,

    /// Package type (e.g., "editor", "tool")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub package_type: Option<String>,

    /// Categories this package belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,

    /// Build stage configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<PackageStage>,

    /// Install stage configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install: Option<PackageStage>,

    /// Update stage configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<PackageStage>,

    /// Uninstall stage configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uninstall: Option<PackageStage>,

    /// Purge stage configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purge: Option<PackageStage>,
}

impl Package {
    /// Merge another package definition into this one
    /// The other package's fields take precedence (used for system-specific overrides)
    pub fn merge(&mut self, other: &Package) {
        // Don't override name, desc, repo, branch if they're the same
        // These should generally be in the base file only
        if !other.name.is_empty() && other.name != self.name {
            self.name = other.name.clone();
        }
        if !other.desc.is_empty() && other.desc != self.desc {
            self.desc = other.desc.clone();
        }
        if !other.repo.is_empty() {
            self.repo = other.repo.clone();
        }
        if !other.branch.is_empty() {
            self.branch = other.branch.clone();
        }

        if other.package_type.is_some() {
            self.package_type = other.package_type.clone();
        }
        if other.categories.is_some() {
            self.categories = other.categories.clone();
        }

        // Merge stages
        if let Some(ref other_build) = other.build {
            if let Some(ref mut build) = self.build {
                build.merge(other_build);
            } else {
                self.build = Some(other_build.clone());
            }
        }

        if let Some(ref other_install) = other.install {
            if let Some(ref mut install) = self.install {
                install.merge(other_install);
            } else {
                self.install = Some(other_install.clone());
            }
        }

        if let Some(ref other_update) = other.update {
            if let Some(ref mut update) = self.update {
                update.merge(other_update);
            } else {
                self.update = Some(other_update.clone());
            }
        }

        if let Some(ref other_uninstall) = other.uninstall {
            if let Some(ref mut uninstall) = self.uninstall {
                uninstall.merge(other_uninstall);
            } else {
                self.uninstall = Some(other_uninstall.clone());
            }
        }

        if let Some(ref other_purge) = other.purge {
            if let Some(ref mut purge) = self.purge {
                purge.merge(other_purge);
            } else {
                self.purge = Some(other_purge.clone());
            }
        }
    }
}

/// Load a package from TOML files
///
/// This function loads a base package file and optionally merges system-specific overrides.
/// It looks for files in the pattern: `{package_name}.toml` and `{package_name}.{system}.toml`
///
/// # Arguments
/// * `package_dir` - Directory containing the package TOML files
/// * `package_name` - Name of the package to load
/// * `system_override` - Optional system-specific override (e.g., "ubuntu", "fedora")
///
/// # Returns
/// A merged Package with system-specific overrides applied if found
pub fn load_package<P: AsRef<Path>>(
    package_dir: P,
    package_name: &str,
    system_override: Option<&str>,
) -> Result<Package, Box<dyn std::error::Error>> {
    let package_dir = package_dir.as_ref();

    // Load base package file
    let base_path = package_dir.join(format!("{}.toml", package_name));
    let base_content = fs::read_to_string(&base_path).map_err(|e| {
        format!(
            "Failed to read base package file '{}': {}",
            base_path.display(),
            e
        )
    })?;

    let mut package: Package = toml::from_str(&base_content).map_err(|e| {
        format!(
            "Failed to parse base package file '{}': {}",
            base_path.display(),
            e
        )
    })?;

    // Load and merge system-specific override if provided
    if let Some(system) = system_override {
        let override_path = package_dir.join(format!("{}.{}.toml", package_name, system));

        if override_path.exists() {
            let override_content = fs::read_to_string(&override_path).map_err(|e| {
                format!(
                    "Failed to read override file '{}': {}",
                    override_path.display(),
                    e
                )
            })?;

            let override_package: Package = toml::from_str(&override_content).map_err(|e| {
                format!(
                    "Failed to parse override file '{}': {}",
                    override_path.display(),
                    e
                )
            })?;

            package.merge(&override_package);
        }
    }

    Ok(package)
}

/// Find all available packages in a repository
///
/// # Arguments
/// * `packages_dir` - Directory containing package subdirectories
///
/// # Returns
/// A vector of package names (directory names)
pub fn list_packages<P: AsRef<Path>>(packages_dir: P) -> Result<Vec<String>, io::Error> {
    let packages_dir = packages_dir.as_ref();
    let mut packages = Vec::new();

    if !packages_dir.exists() {
        return Ok(packages);
    }

    for entry in fs::read_dir(packages_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Skip hidden directories
                if !name.starts_with('.') {
                    packages.push(name.to_string());
                }
            }
        }
    }

    packages.sort();
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_stage_is_privileged() {
        let mut stage = PackageStage::default();
        assert!(!stage.is_privileged());

        stage.privileged = Some(true);
        assert!(stage.is_privileged());

        stage.privileged = None;
        stage.priviledged = Some(true);
        assert!(stage.is_privileged());
    }

    #[test]
    fn test_package_stage_scope_validation() {
        // Test no scope restriction (None) - allows all
        let stage = PackageStage::default();
        assert!(stage.is_scope_allowed("system"));
        assert!(stage.is_scope_allowed("user"));

        // Test empty scope array - allows all
        let stage = PackageStage {
            scope: Some(vec![]),
            ..Default::default()
        };
        assert!(stage.is_scope_allowed("system"));
        assert!(stage.is_scope_allowed("user"));

        // Test system only
        let stage = PackageStage {
            scope: Some(vec!["system".to_string()]),
            ..Default::default()
        };
        assert!(stage.is_scope_allowed("system"));
        assert!(!stage.is_scope_allowed("user"));

        // Test user only
        let stage = PackageStage {
            scope: Some(vec!["user".to_string()]),
            ..Default::default()
        };
        assert!(!stage.is_scope_allowed("system"));
        assert!(stage.is_scope_allowed("user"));

        // Test both scopes allowed
        let stage = PackageStage {
            scope: Some(vec!["system".to_string(), "user".to_string()]),
            ..Default::default()
        };
        assert!(stage.is_scope_allowed("system"));
        assert!(stage.is_scope_allowed("user"));

        // Test case insensitivity
        let stage = PackageStage {
            scope: Some(vec!["SYSTEM".to_string()]),
            ..Default::default()
        };
        assert!(stage.is_scope_allowed("system"));
        assert!(stage.is_scope_allowed("System"));

        // Test invalid scopes are ignored (but shouldn't break)
        let stage = PackageStage {
            scope: Some(vec!["invalid".to_string(), "system".to_string()]),
            ..Default::default()
        };
        assert!(stage.is_scope_allowed("system"));
        assert!(!stage.is_scope_allowed("user"));
        assert!(!stage.is_scope_allowed("invalid"));
    }

    #[test]
    fn test_package_stage_merge() {
        let mut stage1 = PackageStage {
            dependencies: Some(vec!["dep1".to_string()]),
            command: Some("cmd1".to_string()),
            ..Default::default()
        };

        let stage2 = PackageStage {
            dependencies: Some(vec!["dep2".to_string()]),
            artifacts: Some(vec!["artifact1".to_string()]),
            ..Default::default()
        };

        stage1.merge(&stage2);

        assert_eq!(stage1.dependencies, Some(vec!["dep2".to_string()]));
        assert_eq!(stage1.command, Some("cmd1".to_string()));
        assert_eq!(stage1.artifacts, Some(vec!["artifact1".to_string()]));
    }

    #[test]
    fn test_package_stage_merge_all_fields() {
        let mut stage1 = PackageStage::default();

        let stage2 = PackageStage {
            dependencies: Some(vec!["dep1".to_string()]),
            command: Some("make".to_string()),
            artifacts: Some(vec!["bin/app".to_string()]),
            prerequisites: Some(vec!["clean".to_string()]),
            privileged: Some(true),
            priviledged: Some(false),
            scope: Some(vec!["system".to_string()]),
        };

        stage1.merge(&stage2);

        assert_eq!(stage1.dependencies, Some(vec!["dep1".to_string()]));
        assert_eq!(stage1.command, Some("make".to_string()));
        assert_eq!(stage1.artifacts, Some(vec!["bin/app".to_string()]));
        assert_eq!(stage1.prerequisites, Some(vec!["clean".to_string()]));
        assert_eq!(stage1.privileged, Some(true));
        assert_eq!(stage1.priviledged, Some(false));
        assert_eq!(stage1.scope, Some(vec!["system".to_string()]));
    }

    #[test]
    fn test_package_stage_merge_scope() {
        // Test that scope is properly merged
        let mut stage1 = PackageStage {
            scope: Some(vec!["user".to_string()]),
            ..Default::default()
        };

        let stage2 = PackageStage {
            scope: Some(vec!["system".to_string()]),
            ..Default::default()
        };

        stage1.merge(&stage2);
        assert_eq!(stage1.scope, Some(vec!["system".to_string()]));

        // Test that scope is not overridden if not present in override
        let mut stage3 = PackageStage {
            scope: Some(vec!["user".to_string()]),
            ..Default::default()
        };

        let stage4 = PackageStage {
            scope: None,
            ..Default::default()
        };

        stage3.merge(&stage4);
        assert_eq!(stage3.scope, Some(vec!["user".to_string()]));
    }

    #[test]
    fn test_package_merge() {
        let mut base_package = Package {
            name: "test".to_string(),
            desc: "base description".to_string(),
            repo: vec!["https://github.com/test/test.git".to_string()],
            branch: "main".to_string(),
            package_type: Some("tool".to_string()),
            categories: Some(vec!["dev".to_string()]),
            build: Some(PackageStage {
                command: Some("make".to_string()),
                ..Default::default()
            }),
            install: None,
            update: None,
            uninstall: None,
            purge: None,
        };

        let override_package = Package {
            name: String::new(),
            desc: String::new(),
            repo: vec![],
            branch: String::new(),
            package_type: Some("application".to_string()),
            categories: Some(vec!["utility".to_string()]),
            build: Some(PackageStage {
                dependencies: Some(vec!["gcc".to_string()]),
                ..Default::default()
            }),
            install: Some(PackageStage {
                command: Some("make install".to_string()),
                ..Default::default()
            }),
            update: None,
            uninstall: None,
            purge: None,
        };

        base_package.merge(&override_package);

        // Name, desc, repo, branch should remain unchanged when empty in override
        assert_eq!(base_package.name, "test");
        assert_eq!(base_package.desc, "base description");
        assert_eq!(base_package.repo, vec!["https://github.com/test/test.git"]);
        assert_eq!(base_package.branch, "main");

        // Type and categories should be overridden
        assert_eq!(base_package.package_type, Some("application".to_string()));
        assert_eq!(base_package.categories, Some(vec!["utility".to_string()]));

        // Build should be merged
        assert!(base_package.build.is_some());
        let build = base_package.build.unwrap();
        assert_eq!(build.command, Some("make".to_string()));
        assert_eq!(build.dependencies, Some(vec!["gcc".to_string()]));

        // Install should be added
        assert!(base_package.install.is_some());
    }

    #[test]
    fn test_load_package_not_found() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let result = load_package(temp_dir.path(), "nonexistent", None);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read base package file")
        );
    }

    #[test]
    fn test_load_package_invalid_toml() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("test.toml");

        let mut file = fs::File::create(&package_path).unwrap();
        file.write_all(b"invalid toml [[[").unwrap();

        let result = load_package(temp_dir.path(), "test", None);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse base package file")
        );
    }

    #[test]
    fn test_load_package_with_override() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create base package
        let base_toml = r#"
name = "test-package"
desc = "A test package"
repo = ["https://github.com/test/test.git"]
branch = "main"

[build]
command = "make"
"#;
        let base_path = temp_dir.path().join("test-package.toml");
        let mut file = fs::File::create(&base_path).unwrap();
        file.write_all(base_toml.as_bytes()).unwrap();

        // Create system-specific override
        let override_toml = r#"
[build]
dependencies = ["gcc", "make"]
"#;
        let override_path = temp_dir.path().join("test-package.ubuntu.toml");
        let mut file = fs::File::create(&override_path).unwrap();
        file.write_all(override_toml.as_bytes()).unwrap();

        let result = load_package(temp_dir.path(), "test-package", Some("ubuntu"));

        assert!(result.is_ok());
        let package = result.unwrap();
        assert_eq!(package.name, "test-package");
        assert!(package.build.is_some());
        let build = package.build.unwrap();
        assert_eq!(build.command, Some("make".to_string()));
        assert_eq!(
            build.dependencies,
            Some(vec!["gcc".to_string(), "make".to_string()])
        );
    }

    #[test]
    fn test_list_packages() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();

        // Create some package directories
        fs::create_dir(packages_dir.join("vim")).unwrap();
        fs::create_dir(packages_dir.join("neovim")).unwrap();
        fs::create_dir(packages_dir.join("tmux")).unwrap();
        fs::create_dir(packages_dir.join(".hidden")).unwrap(); // Should be ignored

        let result = list_packages(&packages_dir);

        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 3);
        assert!(packages.contains(&"vim".to_string()));
        assert!(packages.contains(&"neovim".to_string()));
        assert!(packages.contains(&"tmux".to_string()));
        assert!(!packages.contains(&".hidden".to_string()));

        // Check that they're sorted
        assert_eq!(packages, vec!["neovim", "tmux", "vim"]);
    }

    #[test]
    fn test_list_packages_empty_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();

        let result = list_packages(&packages_dir);

        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 0);
    }

    #[test]
    fn test_list_packages_nonexistent_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("nonexistent");

        let result = list_packages(&packages_dir);

        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 0);
    }
}
