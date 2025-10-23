use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::io;

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
}

impl PackageStage {
    /// Returns whether this stage requires elevated privileges
    /// Checks both correct and misspelled field names
    pub fn is_privileged(&self) -> bool {
        self.privileged.unwrap_or(false) || self.priviledged.unwrap_or(false)
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
    let base_content = fs::read_to_string(&base_path)
        .map_err(|e| format!("Failed to read base package file '{}': {}", base_path.display(), e))?;
    
    let mut package: Package = toml::from_str(&base_content)
        .map_err(|e| format!("Failed to parse base package file '{}': {}", base_path.display(), e))?;
    
    // Load and merge system-specific override if provided
    if let Some(system) = system_override {
        let override_path = package_dir.join(format!("{}.{}.toml", package_name, system));
        
        if override_path.exists() {
            let override_content = fs::read_to_string(&override_path)
                .map_err(|e| format!("Failed to read override file '{}': {}", override_path.display(), e))?;
            
            let override_package: Package = toml::from_str(&override_content)
                .map_err(|e| format!("Failed to parse override file '{}': {}", override_path.display(), e))?;
            
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
}

