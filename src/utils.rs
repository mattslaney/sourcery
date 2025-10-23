use std::path::PathBuf;

/// Expand tilde (~) in paths to the user's home directory
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            let mut home_path = PathBuf::from(home);
            home_path.push(&path[2..]);
            return home_path;
        }
    }
    PathBuf::from(path)
}

/// Find the first file that exists from a list of locations
pub fn find_file_in_locations(locations: &[&str]) -> Option<PathBuf> {
    for location in locations {
        let path = expand_tilde(location);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

