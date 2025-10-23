/// Integration tests for loading packages and collections from actual repository files
/// These tests require the sourcery-repository to be available
#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use std::path::PathBuf;
    
    fn get_repository_path() -> Option<PathBuf> {
        // Try to find the repository relative to the project root
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.join("sourcery-repository"));
        
        match repo_path {
            Some(ref path) if path.exists() => Some(path.clone()),
            _ => None,
        }
    }
    
    #[test]
    fn test_load_real_package_neovim() {
        let repo_path = match get_repository_path() {
            Some(path) => path,
            None => {
                println!("Skipping test: sourcery-repository not found");
                return;
            }
        };
        
        let packages_dir = repo_path.join("packages").join("neovim");
        
        // Load base neovim package
        let package = package::load_package(&packages_dir, "neovim", None);
        assert!(package.is_ok(), "Failed to load neovim package: {:?}", package.err());
        
        let package = package.unwrap();
        assert_eq!(package.name, "Neovim");
        assert_eq!(package.desc, "Modern replacement for vim");
        assert_eq!(package.branch, "stable");
        assert!(package.build.is_some());
        assert!(package.install.is_some());
    }
    
    #[test]
    fn test_load_real_package_with_override() {
        let repo_path = match get_repository_path() {
            Some(path) => path,
            None => {
                println!("Skipping test: sourcery-repository not found");
                return;
            }
        };
        
        let packages_dir = repo_path.join("packages").join("neovim");
        
        // Load neovim package with ubuntu override
        let package = package::load_package(&packages_dir, "neovim", Some("ubuntu"));
        assert!(package.is_ok(), "Failed to load neovim.ubuntu package: {:?}", package.err());
        
        let package = package.unwrap();
        assert_eq!(package.name, "Neovim");
        
        // Check that the ubuntu-specific build dependencies were merged
        if let Some(build) = package.build {
            assert!(build.dependencies.is_some());
            let deps = build.dependencies.unwrap();
            assert!(deps.contains(&"ninja-build".to_string()));
            assert!(deps.contains(&"build-essential".to_string()));
        } else {
            panic!("Build section should be present");
        }
    }
    
    #[test]
    fn test_load_real_collection_clidev() {
        let repo_path = match get_repository_path() {
            Some(path) => path,
            None => {
                println!("Skipping test: sourcery-repository not found");
                return;
            }
        };
        
        let collection_path = repo_path.join("collections").join("clidev").join("clidev.toml");
        
        let collection = collection::load_collection(&collection_path);
        assert!(collection.is_ok(), "Failed to load clidev collection: {:?}", collection.err());
        
        let collection = collection.unwrap();
        assert_eq!(collection.name, "clidev");
        assert_eq!(collection.collection_type, "collection");
        assert!(collection.packages.contains(&"tmux".to_string()));
        assert!(collection.packages.contains(&"vim".to_string()));
    }
    
    #[test]
    fn test_list_packages() {
        let repo_path = match get_repository_path() {
            Some(path) => path,
            None => {
                println!("Skipping test: sourcery-repository not found");
                return;
            }
        };
        
        let packages_dir = repo_path.join("packages");
        
        let packages = package::list_packages(&packages_dir);
        assert!(packages.is_ok(), "Failed to list packages: {:?}", packages.err());
        
        let packages = packages.unwrap();
        assert!(!packages.is_empty(), "Should have at least one package");
        assert!(packages.contains(&"neovim".to_string()));
        assert!(packages.contains(&"tmux".to_string()));
    }
    
    #[test]
    fn test_list_collections() {
        let repo_path = match get_repository_path() {
            Some(path) => path,
            None => {
                println!("Skipping test: sourcery-repository not found");
                return;
            }
        };
        
        let collections_dir = repo_path.join("collections");
        
        let collections = collection::list_collections(&collections_dir);
        assert!(collections.is_ok(), "Failed to list collections: {:?}", collections.err());
        
        let collections = collections.unwrap();
        assert!(!collections.is_empty(), "Should have at least one collection");
        
        let collection_names: Vec<String> = collections.iter().map(|(name, _)| name.clone()).collect();
        assert!(collection_names.contains(&"clidev".to_string()));
    }
}

