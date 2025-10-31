use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

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
pub fn load_collection<P: AsRef<Path>>(
    collection_path: P,
) -> Result<Collection, Box<dyn std::error::Error>> {
    let collection_path = collection_path.as_ref();

    let content = fs::read_to_string(collection_path).map_err(|e| {
        format!(
            "Failed to read collection file '{}': {}",
            collection_path.display(),
            e
        )
    })?;

    let collection: Collection = toml::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse collection file '{}': {}",
            collection_path.display(),
            e
        )
    })?;

    Ok(collection)
}

/// Find all available collections in a repository
///
/// # Arguments
/// * `collections_dir` - Directory containing collection subdirectories
///
/// # Returns
/// A vector of tuples containing (collection_name, collection_path)
pub fn list_collections<P: AsRef<Path>>(
    collections_dir: P,
) -> Result<Vec<(String, std::path::PathBuf)>, io::Error> {
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

    #[test]
    fn test_load_collection_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let collection_path = temp_dir.path().join("nonexistent.toml");

        let result = load_collection(&collection_path);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read collection file")
        );
    }

    #[test]
    fn test_load_collection_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let collection_path = temp_dir.path().join("test.toml");

        let mut file = fs::File::create(&collection_path).unwrap();
        file.write_all(b"invalid toml [[[").unwrap();

        let result = load_collection(&collection_path);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse collection file")
        );
    }

    #[test]
    fn test_list_collections() {
        let temp_dir = TempDir::new().unwrap();
        let collections_dir = temp_dir.path().join("collections");
        fs::create_dir(&collections_dir).unwrap();

        // Create collection directories with matching TOML files
        let clidev_dir = collections_dir.join("clidev");
        fs::create_dir(&clidev_dir).unwrap();
        let mut file = fs::File::create(clidev_dir.join("clidev.toml")).unwrap();
        file.write_all(b"name = \"clidev\"\ndesc = \"test\"\ntype = \"collection\"\ncategories = []\npackages = []").unwrap();

        let devtools_dir = collections_dir.join("devtools");
        fs::create_dir(&devtools_dir).unwrap();
        let mut file = fs::File::create(devtools_dir.join("devtools.toml")).unwrap();
        file.write_all(b"name = \"devtools\"\ndesc = \"test\"\ntype = \"collection\"\ncategories = []\npackages = []").unwrap();

        // Create a directory without a matching TOML file
        fs::create_dir(collections_dir.join("incomplete")).unwrap();

        // Create a hidden directory (should be ignored)
        let hidden_dir = collections_dir.join(".hidden");
        fs::create_dir(&hidden_dir).unwrap();
        let mut file = fs::File::create(hidden_dir.join(".hidden.toml")).unwrap();
        file.write_all(b"name = \"hidden\"\ndesc = \"test\"\ntype = \"collection\"\ncategories = []\npackages = []").unwrap();

        let result = list_collections(&collections_dir);

        assert!(result.is_ok());
        let collections = result.unwrap();
        assert_eq!(collections.len(), 2);

        let names: Vec<String> = collections.iter().map(|(name, _)| name.clone()).collect();
        assert!(names.contains(&"clidev".to_string()));
        assert!(names.contains(&"devtools".to_string()));
        assert!(!names.contains(&"incomplete".to_string()));
        assert!(!names.contains(&".hidden".to_string()));

        // Check that they're sorted
        assert_eq!(names, vec!["clidev", "devtools"]);
    }

    #[test]
    fn test_list_collections_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let collections_dir = temp_dir.path().join("collections");
        fs::create_dir(&collections_dir).unwrap();

        let result = list_collections(&collections_dir);

        assert!(result.is_ok());
        let collections = result.unwrap();
        assert_eq!(collections.len(), 0);
    }

    #[test]
    fn test_list_collections_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let collections_dir = temp_dir.path().join("nonexistent");

        let result = list_collections(&collections_dir);

        assert!(result.is_ok());
        let collections = result.unwrap();
        assert_eq!(collections.len(), 0);
    }
}
