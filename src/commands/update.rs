use super::{handle_build, handle_install, handle_uninstall};
use crate::config::Config;
use crate::logging;
use crate::messages;
use crate::system::SystemInfo;
use crate::utilities::{load_package, BuildEnvironment, GitRepo, Package, resolve_prerequisites};
use crate::utils;

pub fn handle_update(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    branch: Option<String>,
    tag: Option<String>,
    verbose: bool,
    noconfirm: bool,
) {
    // Update operations may need to check installation scope
    // For now, we don't drop privileges automatically since we may need them later
    messages::msg(format!("Updating package: {}", package));

    if let Some(ref branch) = branch {
        messages::msg(format!("  From branch: {}", branch));
    }
    if let Some(ref tag) = tag {
        messages::msg(format!("  From tag: {}", tag));
    }

    // Step 1: Load package definition
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

    // Step 2: Check if package has update stage
    let update_stage = match &package_def.update {
        Some(stage) => stage,
        None => {
            // No update stage defined - use default update behavior
            messages::msg("No update stage defined, using default update behavior:");
            messages::msg("  1. Build new version");
            messages::msg("  2. Uninstall current version");
            messages::msg("  3. Install new version");
            
            if !noconfirm {
                if !messages::confirm("Proceed with default update?") {
                    messages::msg("Update cancelled.");
                    return;
                }
            }

            // Execute default update: build -> uninstall -> install
            if let Err(e) = execute_default_update(
                config,
                system_info,
                package,
                branch.clone(),
                tag.clone(),
                verbose,
                noconfirm,
            ) {
                messages::failure(format!("Update failed: {}", e));
                return;
            }

            messages::success(format!("Package '{}' updated successfully!", package));
            return;
        }
    };

    // Step 3: Execute prerequisites if any
    if let Some(prerequisites) = &update_stage.prerequisites {
        if !prerequisites.is_empty() {
            messages::msg(format!("Processing {} prerequisite(s)...", prerequisites.len()));

            // Resolve prerequisites with circular dependency detection
            let get_prereqs = |stage_name: &str| {
                get_stage_prerequisites(&package_def, stage_name)
            };

            let stages_to_run = match resolve_prerequisites("update", prerequisites, &get_prereqs) {
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
                    branch.clone(),
                    tag.clone(),
                    verbose,
                    noconfirm,
                ) {
                    messages::failure(format!("Failed to execute prerequisite '{}': {}", stage_name, e));
                    return;
                }
            }
        }
    }

    // Step 4: Execute update command if defined
    if let Some(update_command) = &update_stage.command {
        messages::msg("Executing update stage...");
        
        if let Err(e) = execute_update_stage(
            config,
            package,
            &package_def,
            update_command,
            verbose,
            noconfirm,
        ) {
            messages::failure(format!("Update failed: {}", e));
            return;
        }
    } else {
        messages::msg("Update stage has no command defined - prerequisites only.");
    }

    messages::success(format!("Package '{}' updated successfully!", package));
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
    branch: Option<String>,
    tag: Option<String>,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    match stage_name {
        "build" => {
            // Drop privileges for build if running as root
            utils::ensure_not_root("build", verbose);
            
            handle_build(
                config,
                system_info,
                package,
                branch,
                tag,
                None, // container (use config default)
                false, // local
                false, // chroot
                false, // force
                verbose,
                noconfirm,
            );
            Ok(())
        }
        "install" => {
            handle_install(
                config,
                system_info,
                package,
                branch,
                tag,
                false, // user
                false, // system - let install determine from current state
                verbose,
                noconfirm,
            );
            Ok(())
        }
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
            Err(format!("Unknown prerequisite stage: {}", stage_name))
        }
    }
}

/// Execute the default update behavior: build -> uninstall -> install
fn execute_default_update(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    branch: Option<String>,
    tag: Option<String>,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    // Step 1: Build new version
    messages::msg("Step 1: Building new version...");
    utils::ensure_not_root("build", verbose);
    
    handle_build(
        config,
        system_info,
        package,
        branch.clone(),
        tag.clone(),
        None, // container (use config default)
        false, // local
        false, // chroot
        true, // force rebuild
        verbose,
        noconfirm,
    );

    // Step 2: Uninstall current version
    messages::msg("Step 2: Uninstalling current version...");
    handle_uninstall(
        config,
        system_info,
        package,
        verbose,
        noconfirm,
    );

    // Step 3: Install new version
    messages::msg("Step 3: Installing new version...");
    handle_install(
        config,
        system_info,
        package,
        branch,
        tag,
        false, // user
        false, // system - let install determine from current state
        verbose,
        noconfirm,
    );

    Ok(())
}

/// Execute the update stage command
fn execute_update_stage(
    config: &Config,
    package: &str,
    package_def: &Package,
    update_command: &str,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    if verbose {
        logging::debug(format!("Update command: {}", update_command));
    }

    // Setup environment for update
    let source_dir = config.local_storage_path().join("source").join(package);
    
    if !source_dir.exists() {
        return Err(format!("Source directory not found: {}", source_dir.display()));
    }

    // Get commit hash
    let git_repo = GitRepo::new(&source_dir);
    let commit_hash = git_repo.get_short_commit_hash()
        .unwrap_or_else(|_| "unknown".to_string());

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
        logging::debug("Update environment variables:");
        for (key, value) in &env_vars {
            logging::debug(format!("  {}={}", key, value));
        }
    }

    messages::info(format!("Update command: sh -c '{}'", update_command));

    // Ask for confirmation unless noconfirm is set
    if !noconfirm {
        if !messages::confirm("Proceed with update?") {
            return Err("Update cancelled by user".to_string());
        }
    }

    // Execute update command
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(update_command)
        .current_dir(&source_dir);

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute update command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Update command failed:\n{}", stderr));
    }

    if verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            logging::debug(format!("Update output:\n{}", stdout));
        }
    }

    Ok(())
}
