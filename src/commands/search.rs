use crate::config::Config;
use crate::logging;
use crate::system::SystemInfo;
use crate::utilities::{load_collection, load_package};
use crate::{green, style};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
struct SearchResult {
    name: String,
    repos: Vec<String>, // Repositories where this package/collection was found, in priority order
}

pub fn handle_search(
    config: &Config,
    system_info: &SystemInfo,
    query: &str,
    _fuzzy: bool,
    exact: bool,
    package: bool,
    collection: bool,
    info: bool,
) {
    let search_type = if exact { "exact" } else { "fuzzy" };
    let target = if package {
        "package"
    } else if collection {
        "collection"
    } else {
        "package or collection"
    };

    logging::msg(format!(
        "Searching for {} '{}' using {} search...",
        target,
        query,
        search_type
    ));
    logging::blank();

    // Get repositories sorted by priority
    let repos = config.repositories_by_priority();

    // Store results: key is package/collection name, value is list of repos it was found in
    let mut results: HashMap<String, Vec<String>> = HashMap::new();

    // Search through each repository
    for (repo_name, _repo_info) in &repos {
        let repo_path = config
            .local_storage_path()
            .join("repositories")
            .join(repo_name);

        if !repo_path.exists() {
            continue;
        }

        // Search packages if requested
        if !collection {
            let packages_path = repo_path.join("packages");
            if packages_path.exists() {
                search_in_directory(&packages_path, query, exact, repo_name, &mut results);
            }
        }

        // Search collections if requested
        if !package {
            let collections_path = repo_path.join("collections");
            if collections_path.exists() {
                search_in_directory(&collections_path, query, exact, repo_name, &mut results);
            }
        }
    }

    // Display results
    if results.is_empty() {
        logging::msg("No results found.");
    } else {
        logging::msg(format!("Found {} result(s):", results.len()));
        logging::blank();

        // Convert to SearchResult and sort by name
        let mut search_results: Vec<SearchResult> = results
            .into_iter()
            .map(|(name, repos)| SearchResult { name, repos })
            .collect();
        search_results.sort_by(|a, b| a.name.cmp(&b.name));

        // Display each result
        for result in search_results {
            if info {
                display_info_result(config, system_info, &result, !collection);
            } else {
                display_search_result(&result);
            }
        }
    }
}

fn search_in_directory(
    dir_path: &PathBuf,
    query: &str,
    exact: bool,
    repo_name: &str,
    results: &mut HashMap<String, Vec<String>>,
) {
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Check if this directory name matches the query
                let matches = if exact {
                    dir_name == query
                } else {
                    dir_name.to_lowercase().contains(&query.to_lowercase())
                };

                if matches {
                    results
                        .entry(dir_name.to_string())
                        .or_insert_with(Vec::new)
                        .push(repo_name.to_string());
                }
            }
        }
    }
}

fn display_search_result(result: &SearchResult) {
    let mut output = format!("  {}", result.name);

    if !result.repos.is_empty() {
        output.push_str(" [");
        for (i, repo) in result.repos.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            // First repo (highest priority) is shown in green
            if i == 0 {
                output.push_str(&green!("{}", repo));
            } else {
                output.push_str(repo);
            }
        }
        output.push(']');
    }

    logging::msg(output);
}

fn display_info_result(
    config: &Config,
    system_info: &SystemInfo,
    result: &SearchResult,
    is_package: bool,
) {
    if result.repos.is_empty() {
        return;
    }

    // Use the first repo (highest priority)
    let repo_name = &result.repos[0];
    let repo_path = config
        .local_storage_path()
        .join("repositories")
        .join(repo_name);

    if is_package {
        // Try to load package information
        let package_dir = repo_path.join("packages").join(&result.name);

        if package_dir.exists() {
            // Determine system-specific override
            let system_override = Some(system_info.distro_id.as_str());

            match load_package(&package_dir, &result.name, system_override) {
                Ok(package) => {
                    // Display package name (bold)
                    logging::msg(style!("bold", "{}", package.name));

                    // Display description
                    logging::msg(format!("    {}", package.desc));

                    // Display type and categories on same line
                    let mut meta_info = String::new();

                    if let Some(pkg_type) = &package.package_type {
                        meta_info.push_str(&format!("Type: {}", green!("{}", pkg_type)));
                    }

                    if let Some(categories) = &package.categories {
                        if !categories.is_empty() {
                            if !meta_info.is_empty() {
                                meta_info.push_str("  |  ");
                            }
                            meta_info.push_str(&format!("Categories: {}", categories.join(", ")));
                        }
                    }

                    if !meta_info.is_empty() {
                        logging::msg(format!("    {}", meta_info));
                    }

                    // Show which repo this is from
                    if result.repos.len() > 1 {
                        logging::msg(format!(
                            "    Available in: {}",
                            result
                                .repos
                                .iter()
                                .enumerate()
                                .map(|(i, r)| if i == 0 {
                                    green!("{}", r)
                                } else {
                                    r.to_string()
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    } else {
                        logging::msg(format!("    Repository: {}", green!("{}", repo_name)));
                    }

                    logging::blank();
                }
                Err(e) => {
                    logging::msg(format!("  {} [{}]", result.name, green!("{}", repo_name)));
                    logging::msg(format!("    Error loading package info: {}", e));
                    logging::blank();
                }
            }
        }
    } else {
        // Try to load collection information
        let collection_path = repo_path
            .join("collections")
            .join(&result.name)
            .join(format!("{}.toml", result.name));

        if collection_path.exists() {
            match load_collection(&collection_path) {
                Ok(collection) => {
                    // Display collection name (bold)
                    logging::msg(style!("bold", "{}", collection.name));

                    // Display description
                    logging::msg(format!("    {}", collection.desc));

                    // Display type and categories on same line
                    let mut meta_info = String::new();

                    meta_info.push_str(&format!(
                        "Type: {}",
                        green!("{}", collection.collection_type)
                    ));

                    if !collection.categories.is_empty() {
                        meta_info.push_str(&format!(
                            "  |  Categories: {}",
                            collection.categories.join(", ")
                        ));
                    }

                    logging::msg(format!("    {}", meta_info));

                    // Show packages in collection
                    if !collection.packages.is_empty() {
                        logging::msg(format!("    Packages: {}", collection.packages.join(", ")));
                    }

                    // Show which repo this is from
                    if result.repos.len() > 1 {
                        logging::msg(format!(
                            "    Available in: {}",
                            result
                                .repos
                                .iter()
                                .enumerate()
                                .map(|(i, r)| if i == 0 {
                                    green!("{}", r)
                                } else {
                                    r.to_string()
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    } else {
                        logging::msg(format!("    Repository: {}", green!("{}", repo_name)));
                    }

                    logging::blank();
                }
                Err(e) => {
                    logging::msg(format!("  {} [{}]", result.name, green!("{}", repo_name)));
                    logging::msg(format!("    Error loading collection info: {}", e));
                    logging::blank();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_search_in_directory_exact_match() {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();
        fs::create_dir(packages_dir.join("neovim")).unwrap();
        fs::create_dir(packages_dir.join("vim")).unwrap();
        fs::create_dir(packages_dir.join("tmux")).unwrap();

        let mut results = HashMap::new();
        search_in_directory(&packages_dir, "vim", true, "test-repo", &mut results);

        assert_eq!(results.len(), 1);
        assert!(results.contains_key("vim"));
        assert_eq!(results["vim"], vec!["test-repo"]);
    }

    #[test]
    fn test_search_in_directory_fuzzy_match() {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();
        fs::create_dir(packages_dir.join("neovim")).unwrap();
        fs::create_dir(packages_dir.join("vim")).unwrap();
        fs::create_dir(packages_dir.join("tmux")).unwrap();

        let mut results = HashMap::new();
        search_in_directory(&packages_dir, "vim", false, "test-repo", &mut results);

        // Should match both "vim" and "neovim"
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("vim"));
        assert!(results.contains_key("neovim"));
    }

    #[test]
    fn test_search_in_directory_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();
        fs::create_dir(packages_dir.join("Neovim")).unwrap();

        let mut results = HashMap::new();
        search_in_directory(&packages_dir, "neovim", false, "test-repo", &mut results);

        assert_eq!(results.len(), 1);
        assert!(results.contains_key("Neovim"));
    }

    #[test]
    fn test_search_in_directory_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();
        fs::create_dir(packages_dir.join("vim")).unwrap();

        let mut results = HashMap::new();
        search_in_directory(&packages_dir, "emacs", true, "test-repo", &mut results);

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_in_directory_multiple_repos() {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();
        fs::create_dir(packages_dir.join("vim")).unwrap();

        let mut results = HashMap::new();
        search_in_directory(&packages_dir, "vim", true, "repo1", &mut results);
        search_in_directory(&packages_dir, "vim", true, "repo2", &mut results);

        assert_eq!(results.len(), 1);
        assert_eq!(results["vim"], vec!["repo1", "repo2"]);
    }

    #[test]
    fn test_search_in_directory_ignores_files() {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("packages");
        fs::create_dir(&packages_dir).unwrap();
        fs::create_dir(packages_dir.join("vim")).unwrap();
        fs::File::create(packages_dir.join("readme.txt")).unwrap();

        let mut results = HashMap::new();
        search_in_directory(&packages_dir, "readme", false, "test-repo", &mut results);

        // Should not match files, only directories
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_in_directory_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let packages_dir = temp_dir.path().join("nonexistent");

        let mut results = HashMap::new();
        search_in_directory(&packages_dir, "vim", false, "test-repo", &mut results);

        // Should not panic, just return no results
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_display_search_result_single_repo() {
        let result = SearchResult {
            name: "vim".to_string(),
            repos: vec!["main".to_string()],
        };

        // This will print output, but we're mainly testing it doesn't panic
        display_search_result(&result);
    }

    #[test]
    fn test_display_search_result_multiple_repos() {
        let result = SearchResult {
            name: "vim".to_string(),
            repos: vec!["main".to_string(), "wip".to_string()],
        };

        display_search_result(&result);
    }

    #[test]
    fn test_display_search_result_no_repos() {
        let result = SearchResult {
            name: "vim".to_string(),
            repos: vec![],
        };

        display_search_result(&result);
    }
}
