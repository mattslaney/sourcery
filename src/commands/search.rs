use crate::config::Config;
use crate::system::SystemInfo;
use crate::{green, msg};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
struct SearchResult {
    name: String,
    repos: Vec<String>,  // Repositories where this package/collection was found, in priority order
}

pub fn handle_search(
    config: &Config,
    _system_info: &SystemInfo,
    query: &str,
    _fuzzy: bool,
    exact: bool,
    package: bool,
    collection: bool,
) {
    let search_type = if exact { "exact" } else { "fuzzy" };
    let target = if package {
        "package"
    } else if collection {
        "collection"
    } else {
        "package or collection"
    };
    
    msg!("Searching for {} '{}' using {} search...", target, query, search_type);
    msg!();

    // Get repositories sorted by priority
    let repos = config.repositories_by_priority();
    
    // Store results: key is package/collection name, value is list of repos it was found in
    let mut results: HashMap<String, Vec<String>> = HashMap::new();
    
    // Search through each repository
    for (repo_name, _repo_info) in &repos {
        let repo_path = config.local_storage_path().join("repositories").join(repo_name);
        
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
        msg!("No results found.");
    } else {
        msg!("Found {} result(s):", results.len());
        msg!();
        
        // Convert to SearchResult and sort by name
        let mut search_results: Vec<SearchResult> = results.into_iter()
            .map(|(name, repos)| SearchResult { name, repos })
            .collect();
        search_results.sort_by(|a, b| a.name.cmp(&b.name));
        
        // Display each result
        for result in search_results {
            display_search_result(&result);
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
                let dir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Check if this directory name matches the query
                let matches = if exact {
                    dir_name == query
                } else {
                    dir_name.to_lowercase().contains(&query.to_lowercase())
                };
                
                if matches {
                    results.entry(dir_name.to_string())
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
    
    msg!("{}", output);
}

