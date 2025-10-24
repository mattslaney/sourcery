use std::path::PathBuf;
use std::process::Command;
use nix::unistd::Uid;
use crate::messages;
use crate::logging;

/// Information about the original user when running with sudo
#[derive(Debug, Clone)]
pub struct SudoContext {
    pub uid: u32,
    pub gid: u32,
    pub user: String,
    pub home: String,
}

impl SudoContext {
    /// Detect if we're running as root via sudo and get the original user's context
    pub fn detect() -> Option<Self> {
        // Only relevant if we're currently running as root
        if !Uid::effective().is_root() {
            return None;
        }

        // Check if we have SUDO_UID and SUDO_GID environment variables
        let sudo_uid = std::env::var("SUDO_UID").ok()?;
        let sudo_gid = std::env::var("SUDO_GID").ok()?;
        let sudo_user = std::env::var("SUDO_USER").ok()?;
        
        // Get the original user's HOME directory
        // When sudo is used, it typically sets HOME to root's home
        // We need to reconstruct the original user's home
        let sudo_home = format!("/home/{}", sudo_user);

        Some(SudoContext {
            uid: sudo_uid.parse().ok()?,
            gid: sudo_gid.parse().ok()?,
            user: sudo_user,
            home: sudo_home,
        })
    }

    /// Re-execute the current command as the original user
    /// This will exit the current process and never return
    pub fn reexec_as_user(&self, verbose: bool) -> ! {
        messages::info(format!(
            "This operation doesn't require root privileges - executing as user '{}'",
            self.user
        ));
        
        if verbose {
            logging::debug(format!(
                "Using 'su' to drop privileges (uid: {} -> {})",
                Uid::effective(), self.uid
            ));
        }

        // Get current executable path
        let exe_path = std::env::current_exe()
            .expect("Failed to get current executable path");

        // Collect all command-line arguments (skip argv[0] which is the program name)
        let args: Vec<String> = std::env::args().skip(1).collect();

        // Build the command string
        let command_str = if args.is_empty() {
            exe_path.display().to_string()
        } else {
            format!("{} {}", exe_path.display(), args.join(" "))
        };

        if verbose {
            logging::debug(format!("Executing: su {} -c \"{}\"", self.user, command_str));
        }

        // Use 'su' to switch to the original user
        // Note: We don't use 'su -' (login shell) because that would change the working directory
        // and lose the current environment. We want to preserve the working directory and most env vars.
        let mut cmd = Command::new("su");
        cmd.arg(&self.user)
            .arg("-c")
            .arg(&command_str);

        // Execute and exit with the same exit code
        let status = cmd.status()
            .expect("Failed to execute command as user");

        std::process::exit(status.code().unwrap_or(1));
    }
}

/// Check if running as root and drop privileges if not needed
/// Call this at the start of any command that never needs root
pub fn ensure_not_root(operation: &str, verbose: bool) {
    if let Some(ctx) = SudoContext::detect() {
        messages::caution(format!(
            "Running '{}' with sudo is unnecessary and may cause issues - dropping to user '{}'",
            operation, ctx.user
        ));
        ctx.reexec_as_user(verbose);
    }
}

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
            "/etc/os-release", // Should exist on modern Linux
            "/etc/passwd",     // Also exists, but shouldn't be returned
        ];

        let result = find_file_in_locations(&locations);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), PathBuf::from("/etc/os-release"));
    }
}
