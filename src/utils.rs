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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_expand_tilde_with_home() {
        // Save original HOME
        let original_home = env::var_os("HOME");
        
        // Set a test HOME
        unsafe {
            env::set_var("HOME", "/test/home");
        }
        
        let result = expand_tilde("~/Documents/file.txt");
        assert_eq!(result, PathBuf::from("/test/home/Documents/file.txt"));
        
        // Restore original HOME
        if let Some(home) = original_home {
            unsafe {
                env::set_var("HOME", home);
            }
        }
    }

    #[test]
    fn test_expand_tilde_without_tilde() {
        let result = expand_tilde("/absolute/path/file.txt");
        assert_eq!(result, PathBuf::from("/absolute/path/file.txt"));
    }

    #[test]
    fn test_expand_tilde_relative_path() {
        let result = expand_tilde("relative/path/file.txt");
        assert_eq!(result, PathBuf::from("relative/path/file.txt"));
    }

    #[test]
    fn test_find_file_in_locations_none_exist() {
        let locations = vec![
            "/nonexistent/path1",
            "/nonexistent/path2",
            "/nonexistent/path3",
        ];
        
        let result = find_file_in_locations(&locations);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_file_in_locations_finds_first() {
        // This test uses actual files that should exist on most Linux systems
        let locations = vec![
            "/nonexistent/path",
            "/etc/os-release",  // Should exist on modern Linux
            "/etc/passwd",      // Also exists, but shouldn't be returned
        ];
        
        let result = find_file_in_locations(&locations);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), PathBuf::from("/etc/os-release"));
    }
}

