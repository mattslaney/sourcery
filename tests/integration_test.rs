use std::process::Command;

fn get_binary_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/sourcery", manifest_dir)
}

#[test]
fn test_health_command() {
    let output = Command::new(get_binary_path())
        .arg("--health")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for expected system information output
    assert!(stdout.contains("System Health Check"));
    assert!(stdout.contains("Architecture:"));
    assert!(stdout.contains("Distribution:"));

    // Check for dependency checks section
    assert!(stdout.contains("Dependency Checks:"));

    // Check that git check is performed (should show either ✓ or ✗)
    assert!(stdout.contains("Git is installed") || stdout.contains("Git is not installed"));

    // Check that container runtime check is performed
    assert!(
        stdout.contains("Container runtime found") || stdout.contains("No container runtime found")
    );

    // Check that repository checks are performed
    assert!(
        stdout.contains("Repository")
            && (stdout.contains("exists") || stdout.contains("not found"))
    );
}

#[test]
fn test_help_command() {
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for expected help text
    assert!(stdout.contains("A package manager for maintaining system packages from source"));
    assert!(stdout.contains("--update"));
    assert!(stdout.contains("--clean"));
    assert!(stdout.contains("--health"));
}

#[test]
fn test_list_command_requires_flag() {
    let output = Command::new(get_binary_path())
        .arg("list")
        .output()
        .expect("Failed to execute command");

    // Should fail because --installed or --upgradable is required
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--installed") || stderr.contains("--upgradable"));
}

#[test]
fn test_search_command() {
    let output = Command::new(get_binary_path())
        .arg("search")
        .arg("test-package")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Searching for"));
    assert!(stdout.contains("test-package"));
}

#[test]
fn test_build_command() {
    let output = Command::new(get_binary_path())
        .arg("build")
        .arg("test-package")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Building package: test-package"));
}

#[test]
fn test_no_command_shows_error() {
    let output = Command::new(get_binary_path())
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No command specified") || stderr.contains("--help"));
}

#[test]
fn test_version_flag() {
    let output = Command::new(get_binary_path())
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sourcery"));
}

#[test]
fn test_build_with_options() {
    let output = Command::new(get_binary_path())
        .arg("build")
        .arg("test-package")
        .arg("--verbose")
        .arg("--confirm")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Building package: test-package"));
    // The build will fail because the package doesn't exist, but we should at least
    // see the initial output
}

#[test]
fn test_install_command() {
    let output = Command::new(get_binary_path())
        .arg("install")
        .arg("test-package")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installing package: test-package"));
}
