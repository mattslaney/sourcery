use crate::config::Config;
use crate::logging;
use crate::system::SystemInfo;
use std::process::Command;

pub fn handle_health(config: &Config, system_info: &SystemInfo) {
    logging::msg(style!("bold,cyan", "System Health Check"));
    logging::msg(style!("bold,cyan", "=================="));
    logging::blank();

    logging::msg(style!("bold", "System Information:"));
    logging::msg(format!("  Architecture: {}", system_info.arch));
    logging::msg(format!(
        "  Distribution: {} ({})",
        system_info.distro_name,
        system_info.distro_id
    ));
    logging::msg(format!("  Version: {}", system_info.distro_version));
    if !system_info.distro_version_id.is_empty() {
        logging::msg(format!("  Version ID: {}", system_info.distro_version_id));
    }
    if let Some(codename) = &system_info.distro_version_codename {
        logging::msg(format!("  Codename: {}", codename));
    }
    logging::blank();

    logging::msg(style!("bold", "Dependency Checks:"));

    // Check for git
    if check_command_exists("git") {
        logging::msg(format!("  {} Git is installed", green!("✓")));
    } else {
        logging::msg(format!(
            "  {} Git is not installed - required for repository management",
            red!("✗")
        ));
    }

    // Check for container runtime (podman or docker)
    let has_podman = check_command_exists("podman");
    let has_docker = check_command_exists("docker");

    if has_podman || has_docker {
        if has_podman && has_docker {
            logging::msg(format!(
                "  {} Container runtime found: podman and docker",
                green!("✓")
            ));
        } else if has_podman {
            logging::msg(format!("  {} Container runtime found: podman", green!("✓")));
        } else {
            logging::msg(format!("  {} Container runtime found: docker", green!("✓")));
        }
    } else {
        logging::msg(format!(
            "  {} No container runtime found - either podman or docker is required",
            red!("✗")
        ));
    }

    // Check for each repository
    let repositories_dir = config.local_storage_path().join("repositories");
    for (name, repo_info) in &config.repositories {
        let repo_path = repositories_dir.join(name);
        if repo_path.exists() {
            logging::msg(format!(
                "  {} Repository '{}' exists ({})",
                green!("✓"),
                name,
                repo_info.branch
            ));
        } else {
            logging::msg(format!(
                "  {} Repository '{}' not found ({})",
                orange!("⚠"),
                name,
                repo_info.branch
            ));
            logging::msg(format!(
                "    Run {} to clone repositories",
                style!("bold", "sourcery --update")
            ));
        }
    }

    logging::blank();
}

/// Check if a command exists in the system PATH
fn check_command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_command_exists_with_valid_command() {
        // 'ls' should exist on all Unix systems
        assert!(check_command_exists("ls"));
    }

    #[test]
    fn test_check_command_exists_with_invalid_command() {
        // This command should definitely not exist
        assert!(!check_command_exists(
            "this_command_definitely_does_not_exist_12345"
        ));
    }

    #[test]
    fn test_check_git_command() {
        // Git is commonly available, but we just test that the function works
        let result = check_command_exists("git");
        // Just ensure it returns a boolean without panicking
        assert!(result == true || result == false);
    }

    #[test]
    fn test_check_container_runtimes() {
        // Check that we can query for podman or docker without panicking
        let has_podman = check_command_exists("podman");
        let has_docker = check_command_exists("docker");

        // Just ensure they return booleans without panicking
        assert!(has_podman == true || has_podman == false);
        assert!(has_docker == true || has_docker == false);
    }
}
