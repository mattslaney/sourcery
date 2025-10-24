use super::handle_uninstall;
use crate::config::Config;
use crate::logging;
use crate::messages;
use crate::system::SystemInfo;
use crate::utilities::{load_package, BuildEnvironment, GitRepo, Package, resolve_prerequisites};
use std::fs;

pub fn handle_purge(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    verbose: bool,
    noconfirm: bool,
) {
    // Purge may need privileges if package was installed system-wide
    // So we don't automatically drop privileges here
    messages::msg(format!("Purging package: {}", package));
    messages::caution("This will remove the package and all associated data!");

    // Step 1: Load package definition
    messages::msg("Loading package definition...");
    
    let package_def = match load_package_definition(config, package, &system_info.distro_id, verbose) {
        Ok(pkg) => pkg,
        Err(e) => {
            messages::caution(format!("Failed to load package definition: {}", e));
            messages::msg("Continuing with basic purge (remove source, artifacts, logs)...");
            
            // Perform basic purge even without package definition
            if let Err(e) = execute_basic_purge(config, package, verbose, noconfirm) {
                messages::failure(format!("Purge failed: {}", e));
                return;
            }
            
            messages::success(format!("Package '{}' purged (basic cleanup)!", package));
            return;
        }
    };

    if verbose {
        logging::debug(format!("Package loaded: {} - {}", package_def.name, package_def.desc));
    }

    // Step 2: Check if package has purge stage
    let purge_stage = package_def.purge.as_ref();

    // Step 3: Execute prerequisites if any
    if let Some(stage) = purge_stage {
        if let Some(prerequisites) = &stage.prerequisites {
            if !prerequisites.is_empty() {
                messages::msg(format!("Processing {} prerequisite(s)...", prerequisites.len()));

                // Resolve prerequisites with circular dependency detection
                let get_prereqs = |stage_name: &str| {
                    get_stage_prerequisites(&package_def, stage_name)
                };

                let stages_to_run = match resolve_prerequisites("purge", prerequisites, &get_prereqs) {
                    Ok(stages) => stages,
                    Err(e) => {
                        messages::failure(format!("Failed to resolve prerequisites: {}", e));
                        return;
                    }
                };

                if verbose {
                    logging::debug(format!("Resolved prerequisite chain: {:?}", stages_to_run));
                }

                // Execute each prerequisite stage
                for stage_name in &stages_to_run {
                    messages::msg(format!("  Running prerequisite: {}", stage_name));
                    
                    if let Err(e) = execute_prerequisite_stage(
                        config,
                        system_info,
                        package,
                        stage_name,
                        verbose,
                        noconfirm,
                    ) {
                        messages::failure(format!("Failed to execute prerequisite '{}': {}", stage_name, e));
                        return;
                    }
                }
            }
        }
    }

    // Step 4: Execute purge command if defined
    if let Some(stage) = purge_stage {
        if let Some(purge_command) = &stage.command {
            messages::msg("Executing purge stage command...");
            
            if let Err(e) = execute_purge_stage(
                config,
                package,
                &package_def,
                purge_command,
                verbose,
                noconfirm,
            ) {
                messages::failure(format!("Purge command failed: {}", e));
                return;
            }
        }
    }

    // Step 5: Remove sourcery-managed directories (source, artifacts, logs)
    messages::msg("Removing sourcery data directories...");
    
    if let Err(e) = remove_package_directories(config, package, verbose, noconfirm) {
        messages::failure(format!("Failed to remove directories: {}", e));
        return;
    }

    messages::success(format!("Package '{}' purged successfully!", package));
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

/// Get prerequisites for a given stage in a package
fn get_stage_prerequisites(package: &Package, stage_name: &str) -> Option<Vec<String>> {
    let stage = match stage_name {
        "build" => package.build.as_ref(),
        "install" => package.install.as_ref(),
        "update" => package.update.as_ref(),
        "uninstall" => package.uninstall.as_ref(),
        "purge" => package.purge.as_ref(),
        _ => None,
    };

    stage.and_then(|s| s.prerequisites.clone())
}

/// Execute a prerequisite stage by name
fn execute_prerequisite_stage(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    stage_name: &str,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    match stage_name {
        "uninstall" => {
            handle_uninstall(
                config,
                system_info,
                package,
                verbose,
                noconfirm,
            );
            Ok(())
        }
        _ => {
            messages::caution(format!("Unsupported purge prerequisite: {}", stage_name));
            messages::msg("Continuing with purge...");
            Ok(())
        }
    }
}

/// Execute the purge stage command
fn execute_purge_stage(
    config: &Config,
    package: &str,
    package_def: &Package,
    purge_command: &str,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    if verbose {
        logging::debug(format!("Purge command: {}", purge_command));
    }

    messages::caution("About to execute package-defined purge command:");
    messages::info(format!("  {}", purge_command));
    messages::caution("This may remove user configuration and data!");

    // Ask for confirmation unless noconfirm is set
    if !noconfirm {
        if !messages::confirm("Proceed with purge command?") {
            return Err("Purge cancelled by user".to_string());
        }
    }

    // Setup environment for purge
    let source_dir = config.local_storage_path().join("source").join(package);
    
    // Get commit hash if source exists
    let commit_hash = if source_dir.exists() {
        let git_repo = GitRepo::new(&source_dir);
        git_repo.get_short_commit_hash()
            .unwrap_or_else(|_| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    // Determine branch
    let branch = if !package_def.branch.is_empty() {
        Some(package_def.branch.clone())
    } else {
        None
    };

    // Create build environment for variable expansion
    let install_prefix = config.system_install_path(); // or detect from current install
    let build_env = BuildEnvironment::new(
        package.to_string(),
        commit_hash,
        branch,
        config.local_storage_path(),
        install_prefix,
    );

    // Prepare environment variables
    let env_vars = build_env.get_env_vars();

    if verbose {
        logging::debug("Purge environment variables:");
        for (key, value) in &env_vars {
            logging::debug(format!("  {}={}", key, value));
        }
    }

    // Execute purge command
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(purge_command);

    // Use source dir if it exists
    if source_dir.exists() {
        cmd.current_dir(&source_dir);
    }

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute purge command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Purge command failed:\n{}", stderr));
    }

    if verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            logging::debug(format!("Purge output:\n{}", stdout));
        }
    }

    Ok(())
}

/// Remove package directories (source, artifacts, logs)
fn remove_package_directories(
    config: &Config,
    package: &str,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    let base_dir = config.local_storage_path();
    
    let source_dir = base_dir.join("source").join(package);
    let artifacts_dir = base_dir.join("artifacts").join(package);
    let logs_dir = base_dir.join("logs").join("packages").join(package);

    let mut dirs_to_remove = Vec::new();
    
    if source_dir.exists() {
        dirs_to_remove.push(("Source", source_dir.clone()));
    }
    if artifacts_dir.exists() {
        dirs_to_remove.push(("Artifacts", artifacts_dir.clone()));
    }
    if logs_dir.exists() {
        dirs_to_remove.push(("Logs", logs_dir.clone()));
    }

    if dirs_to_remove.is_empty() {
        messages::msg("No directories to remove.");
        return Ok(());
    }

    messages::msg("The following directories will be removed:");
    for (label, path) in &dirs_to_remove {
        messages::msg(format!("  {}: {}", label, path.display()));
    }

    if !noconfirm {
        if !messages::confirm("Remove these directories?") {
            return Err("Directory removal cancelled by user".to_string());
        }
    }

    for (label, path) in &dirs_to_remove {
        if verbose {
            logging::debug(format!("Removing {}: {}", label, path.display()));
        }
        
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove {} directory: {}", label, e))?;
        
        messages::msg(format!("  ✓ Removed {} directory", label));
    }

    Ok(())
}

/// Execute basic purge when package definition is not available
fn execute_basic_purge(
    config: &Config,
    package: &str,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    messages::msg("Performing basic purge (remove source, artifacts, and logs)...");
    
    remove_package_directories(config, package, verbose, noconfirm)
}
