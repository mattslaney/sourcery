use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/sourcery", manifest_dir)
}

fn get_test_config_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/config/sourcery.test.toml", manifest_dir)
}

fn setup_test_dirs() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let test_data_dir = PathBuf::from(manifest_dir).join("test_data");
    
    // Clean up any existing test data
    if test_data_dir.exists() {
        let _ = fs::remove_dir_all(&test_data_dir);
    }
    
    // Create fresh test directories
    let _ = fs::create_dir_all(test_data_dir.join("local_storage"));
    let _ = fs::create_dir_all(test_data_dir.join("user_bin"));
    let _ = fs::create_dir_all(test_data_dir.join("system_bin"));
}

fn run_with_test_config(args: &[&str]) -> std::process::Output {
    setup_test_dirs();
    
    Command::new(get_binary_path())
        .args(args)
        .env("SOURCERY_CONFIG_PATH", get_test_config_path())
        .output()
        .expect("Failed to execute command")
}

#[test]
fn test_health_command() {
    let output = run_with_test_config(&["--health"]);

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
    // Help command doesn't load config, so we can use the binary directly
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
    // List command without flags fails before loading config
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
    let output = run_with_test_config(&["search", "test-package"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Searching for"));
    assert!(stdout.contains("test-package"));
}

#[test]
fn test_build_command() {
    let output = run_with_test_config(&["build", "test-package"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Building package: test-package"));
}

#[test]
fn test_no_command_shows_error() {
    // No command fails before loading config
    let output = Command::new(get_binary_path())
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No command specified") || stderr.contains("--help"));
}

#[test]
fn test_version_flag() {
    // Version flag doesn't load config
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
    let output = run_with_test_config(&["build", "test-package", "--verbose", "--force"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Building package: test-package"));
    // The build will fail because the package doesn't exist, but we should at least
    // see the initial output
}

#[test]
fn test_install_command() {
    let output = run_with_test_config(&["install", "test-package"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installing package: test-package"));
}

#[test]
fn test_clean_command() {
    let output = run_with_test_config(&["--clean", "--noconfirm"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // The output should contain either "Cleaning sources and artifacts" or "Nothing to clean"
    assert!(
        stdout.contains("Cleaning sources and artifacts") 
        || stdout.contains("Nothing to clean")
    );
}

#[test]
fn test_uninstall_command() {
    let output = run_with_test_config(&["uninstall", "test-package"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Uninstalling package: test-package"));
    // The command will fail to find the package, but we should at least
    // see the initial output
}
