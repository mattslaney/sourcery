use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::io;

/// Represents a collection of packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// Collection name
    pub name: String,
    
    /// Collection description
    pub desc: String,
    
    /// Collection type (typically "collection")
    #[serde(rename = "type")]
    pub collection_type: String,
    
    /// Categories this collection belongs to
    pub categories: Vec<String>,
    
    /// List of package names included in this collection
    pub packages: Vec<String>,
}

/// Load a collection from a TOML file
/// 
/// # Arguments
/// * `collection_path` - Path to the collection TOML file
/// 
/// # Returns
/// A Collection struct parsed from the TOML file
pub fn load_collection<P: AsRef<Path>>(collection_path: P) -> Result<Collection, Box<dyn std::error::Error>> {
    let collection_path = collection_path.as_ref();
    
    let content = fs::read_to_string(collection_path)
        .map_err(|e| format!("Failed to read collection file '{}': {}", collection_path.display(), e))?;
    
    let collection: Collection = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse collection file '{}': {}", collection_path.display(), e))?;
    
    Ok(collection)
}

/// Find all available collections in a repository
/// 
/// # Arguments
/// * `collections_dir` - Directory containing collection subdirectories
/// 
/// # Returns
/// A vector of tuples containing (collection_name, collection_path)
pub fn list_collections<P: AsRef<Path>>(collections_dir: P) -> Result<Vec<(String, std::path::PathBuf)>, io::Error> {
    let collections_dir = collections_dir.as_ref();
    let mut collections = Vec::new();
    
    if !collections_dir.exists() {
        return Ok(collections);
    }
    
    for entry in fs::read_dir(collections_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Skip hidden directories
                if !name.starts_with('.') {
                    // Look for a TOML file with the same name as the directory
                    let collection_file = path.join(format!("{}.toml", name));
                    if collection_file.exists() {
                        collections.push((name.to_string(), collection_file));
                    }
                }
            }
        }
    }
    
    collections.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(collections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_load_collection() {
        let temp_dir = TempDir::new().unwrap();
        let collection_path = temp_dir.path().join("test.toml");
        
        let toml_content = r#"
name = "test-collection"
desc = "A test collection"
type = "collection"
categories = ["test", "development"]
packages = ["package1", "package2"]
"#;
        
        let mut file = fs::File::create(&collection_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();
        
        let collection = load_collection(&collection_path).unwrap();
        
        assert_eq!(collection.name, "test-collection");
        assert_eq!(collection.desc, "A test collection");
        assert_eq!(collection.collection_type, "collection");
        assert_eq!(collection.categories, vec!["test", "development"]);
        assert_eq!(collection.packages, vec!["package1", "package2"]);
    }
}

