use crate::config::Config;
use crate::logging;
use crate::messages;
use crate::system::SystemInfo;
use crate::utilities::{load_package, BuildEnvironment, GitRepo, Package};
use nix::unistd::Uid;
use std::path::PathBuf;

pub fn handle_uninstall(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    verbose: bool,
    noconfirm: bool,
) {
    messages::msg(format!("Uninstalling package: {}", package));

    // Step 1: Detect where the package is installed
    let (install_scope, install_prefix) = detect_installation_scope(config, package, verbose);
    
    messages::msg(format!("  Install scope: {} ({})", install_scope, install_prefix.display()));

    // Check if we need elevated privileges for system uninstall
    if install_scope == "system" && !is_running_as_root() {
        messages::failure("System uninstall requires elevated privileges.");
        messages::msg("Please re-run this command with sudo:");
        messages::msg(format!("  sudo sourcery uninstall {}", package));
        return;
    }

    // Step 2: Load package definition
    messages::msg("Loading package definition...");
    
    let package_def = match load_package_definition(config, package, &system_info.distro_id, verbose) {
        Ok(pkg) => pkg,
        Err(e) => {
            messages::failure(format!("Failed to load package: {}", e));
            return;
        }
    };

    if verbose {
        logging::debug(format!("Package loaded: {} - {}", package_def.name, package_def.desc));
    }

    // Step 3: Check if package has uninstall stage
    let uninstall_stage = match &package_def.uninstall {
        Some(stage) => stage,
        None => {
            messages::caution(format!("Package '{}' does not have an uninstall stage defined", package));
            messages::msg("The package was likely installed by copying files.");
            
            // Offer to remove common installation files
            if !offer_basic_uninstall(package, &install_prefix, noconfirm, verbose) {
                return;
            }
            return;
        }
    };

    // Step 4: Check if uninstall command is defined
    if uninstall_stage.command.is_none() {
        messages::failure(format!("Package '{}' has an uninstall stage but no command defined", package));
        return;
    }

    // Step 5: Setup environment for uninstall
    messages::msg("Preparing uninstall environment...");
    
    // Get the source directory and commit hash
    let source_dir = config.local_storage_path().join("source").join(package);
    
    if !source_dir.exists() {
        messages::caution(format!("Source directory not found: {}", source_dir.display()));
        messages::msg("This may be normal if the source was cleaned up after installation.");
        messages::msg("Attempting uninstall anyway...");
    }

    // Try to get commit hash, use "unknown" if not available
    let commit_hash = if source_dir.exists() {
        let git_repo = GitRepo::new(&source_dir);
        match git_repo.get_short_commit_hash() {
            Ok(hash) => hash,
            Err(_) => {
                if verbose {
                    logging::debug("Could not get commit hash, using 'unknown'");
                }
                "unknown".to_string()
            }
        }
    } else {
        "unknown".to_string()
    };

    if verbose && commit_hash != "unknown" {
        logging::debug(format!("Using commit: {}", commit_hash));
    }

    // Get branch from package definition
    let branch = if !package_def.branch.is_empty() {
        Some(package_def.branch.clone())
    } else {
        None
    };

    let build_env = BuildEnvironment::new(
        package.to_string(),
        commit_hash.clone(),
        branch,
        config.local_storage_path(),
        install_prefix.clone(),
    );

    // Step 6: Execute uninstall command
    messages::msg("Executing uninstall stage...");
    
    if let Err(e) = execute_uninstall_stage(&build_env, uninstall_stage, verbose, noconfirm) {
        messages::failure(format!("Uninstall failed: {}", e));
        return;
    }

    messages::success(format!("Package '{}' uninstalled successfully from {}!", package, install_prefix.display()));
}

/// Detect where the package is installed (system or user scope)
/// Returns (scope_name, install_prefix)
fn detect_installation_scope(config: &Config, package: &str, verbose: bool) -> (String, PathBuf) {
    let system_bin = config.system_install_path().join("bin").join(package);
    let user_bin = config.user_install_path().join("bin").join(package);

    if verbose {
        logging::debug(format!("Checking system location: {}", system_bin.display()));
        logging::debug(format!("Checking user location: {}", user_bin.display()));
    }

    // Check if installed in system location
    if system_bin.exists() {
        if verbose {
            logging::debug("Package found in system location");
        }
        return ("system".to_string(), config.system_install_path());
    }

    // Check if installed in user location
    if user_bin.exists() {
        if verbose {
            logging::debug("Package found in user location");
        }
        return ("user".to_string(), config.user_install_path());
    }

    // Default to user if not found (may have been partially installed or in a different location)
    if verbose {
        logging::debug("Package not found in standard locations, defaulting to user scope");
    }
    ("user".to_string(), config.user_install_path())
}

/// Load package definition with system-specific overrides
fn load_package_definition(
    config: &Config,
    package_name: &str,
    distro_id: &str,
    verbose: bool,
) -> Result<Package, String> {
    // Find package in repositories (by priority)
    for (repo_name, _repo_info) in config.repositories_by_priority() {
        let repo_dir = config.local_storage_path().join("repositories").join(&repo_name);
        let packages_dir = repo_dir.join("packages").join(package_name);

        if !packages_dir.exists() {
            continue;
        }

        if verbose {
            logging::debug(format!("Found package in repository: {}", repo_name));
        }

        return load_package(&packages_dir, package_name, Some(distro_id))
            .map_err(|e| format!("Failed to load package from {}: {}", repo_name, e));
    }

    Err(format!("Package '{}' not found in any repository", package_name))
}

/// Execute the uninstall stage
fn execute_uninstall_stage(
    build_env: &BuildEnvironment,
    uninstall_stage: &crate::utilities::PackageStage,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    let uninstall_command = match &uninstall_stage.command {
        Some(cmd) => cmd,
        None => {
            return Err("Uninstall stage does not have a command defined".to_string());
        }
    };

    if verbose {
        logging::debug(format!("Uninstall command: {}", uninstall_command));
    }

    // Prepare environment variables
    let env_vars = build_env.get_env_vars();

    if verbose {
        logging::debug("Uninstall environment variables:");
        for (key, value) in &env_vars {
            logging::debug(format!("  {}={}", key, value));
        }
    }

    messages::info(format!("Uninstall command: sh -c '{}'", uninstall_command));
    messages::info(format!("Install prefix: {}", build_env.install_prefix.display()));

    // Ask for confirmation unless noconfirm is set
    if !noconfirm {
        if !messages::confirm("Proceed with uninstallation?") {
            return Err("Uninstallation cancelled by user".to_string());
        }
    }

    // Execute uninstall command
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(uninstall_command);

    // Use source dir if it exists, otherwise use a temp dir or current dir
    if build_env.source_dir.exists() {
        cmd.current_dir(&build_env.source_dir);
    }

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute uninstall command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Uninstall command failed:\n{}", stderr));
    }

    if verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            logging::debug(format!("Uninstall output:\n{}", stdout));
        }
    }

    Ok(())
}

/// Offer basic uninstall when no uninstall stage is defined
/// Returns true if uninstall was performed or attempted, false if cancelled
fn offer_basic_uninstall(
    package: &str,
    install_prefix: &PathBuf,
    noconfirm: bool,
    _verbose: bool,
) -> bool {
    let bin_path = install_prefix.join("bin").join(package);
    
    if !bin_path.exists() {
        messages::msg(format!("Binary not found at expected location: {}", bin_path.display()));
        messages::msg("Package may already be uninstalled or installed in a different location.");
        return false;
    }

    messages::msg(format!("Found binary: {}", bin_path.display()));
    messages::msg("Offering basic uninstall (remove binary only)...");

    if !noconfirm {
        if !messages::confirm("Remove this binary?") {
            messages::msg("Uninstallation cancelled.");
            return false;
        }
    }

    match std::fs::remove_file(&bin_path) {
        Ok(_) => {
            messages::success(format!("Removed: {}", bin_path.display()));
            messages::caution("Note: Only the main binary was removed. Configuration files and other data may remain.");
            true
        }
        Err(e) => {
            messages::failure(format!("Failed to remove binary: {}", e));
            false
        }
    }
}

/// Check if the current process is running as root
fn is_running_as_root() -> bool {
    Uid::effective().is_root()
}
