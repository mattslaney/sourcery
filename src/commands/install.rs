use super::handle_build;
use crate::config::Config;
use crate::logging;
use crate::messages;
use crate::system::SystemInfo;
use crate::utilities::{
    load_package, BuildEnvironment, GitRepo, Package, resolve_prerequisites,
};
use nix::unistd::Uid;

#[allow(clippy::too_many_arguments)]
pub fn handle_install(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    branch: Option<String>,
    _tag: Option<String>,
    user: bool,
    system: bool,
    verbose: bool,
    noconfirm: bool,
) {
    messages::msg(format!("Installing package: {}", package));

    // Determine install scope
    let use_system = if system {
        true
    } else if user {
        false
    } else {
        // Default to user install
        false
    };

    let install_prefix = if use_system {
        config.system_install_path()
    } else {
        config.user_install_path()
    };

    let scope_str = if use_system { "system" } else { "user" };
    messages::msg(format!("  Install scope: {} ({})", scope_str, install_prefix.display()));

    // Check if we need elevated privileges
    if use_system && !is_running_as_root() {
        messages::failure("System install requires elevated privileges.");
        messages::msg("Please re-run this command with sudo:");
        messages::msg(format!("  sudo sourcery install {} --system", package));
        return;
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

    // Check if package has install stage
    let install_stage = match &package_def.install {
        Some(stage) => stage,
        None => {
            messages::failure(format!("Package '{}' does not have an install stage defined", package));
            return;
        }
    };

    // Check if the requested installation scope is allowed
    let requested_scope = if use_system { "system" } else { "user" };
    if !install_stage.is_scope_allowed(requested_scope) {
        messages::failure(format!(
            "Package '{}' cannot be installed in '{}' scope.",
            package, requested_scope
        ));
        
        // Provide helpful message about allowed scopes
        if let Some(allowed_scopes) = &install_stage.scope {
            let valid_scopes: Vec<String> = allowed_scopes
                .iter()
                .filter(|s| s.to_lowercase() == "system" || s.to_lowercase() == "user")
                .map(|s| s.clone())
                .collect();
            
            if !valid_scopes.is_empty() {
                messages::msg(format!("Allowed scope(s): {}", valid_scopes.join(", ")));
                
                // Suggest correct command
                if valid_scopes.len() == 1 {
                    let scope = valid_scopes[0].to_lowercase();
                    messages::msg(format!(
                        "Please use: sourcery install {} --{}",
                        package, scope
                    ));
                }
            }
        }
        return;
    }

    // Step 2: Execute prerequisites FIRST (they may create the source directory)
    if let Some(prerequisites) = &install_stage.prerequisites {
        if !prerequisites.is_empty() {
            messages::msg(format!("Processing {} prerequisite(s)...", prerequisites.len()));

            // Resolve prerequisites with circular dependency detection
            let get_prereqs = |stage_name: &str| {
                get_stage_prerequisites(&package_def, stage_name)
            };

            let stages_to_run = match resolve_prerequisites("install", prerequisites, &get_prereqs) {
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
                
                // For "build" prerequisite, call the build command directly
                if stage_name == "build" {
                    if let Err(e) = execute_build_prerequisite(
                        config,
                        system_info,
                        package,
                        branch.clone(),
                        verbose,
                        noconfirm,
                    ) {
                        messages::failure(format!("Failed to execute build prerequisite: {}", e));
                        return;
                    }
                } else {
                    // For other stages, we need to create a temporary build env
                    // This is a limitation we'll address if we encounter it
                    messages::msg(format!(
                        "  Note: Generic prerequisite execution for '{}' not yet fully implemented",
                        stage_name
                    ));
                }
            }
        }
    }

    // Step 3: NOW check that source repository exists (after prerequisites)
    messages::msg("Verifying source repository...");
    
    let source_dir = config.local_storage_path().join("source").join(package);
    
    if !source_dir.exists() {
        messages::failure(format!("Source directory does not exist: {}", source_dir.display()));
        messages::msg("Prerequisites completed but source directory was not created.");
        return;
    }

    let git_repo = GitRepo::new(&source_dir);
    let commit_hash = match git_repo.get_short_commit_hash() {
        Ok(hash) => hash,
        Err(e) => {
            messages::failure(format!("Failed to get commit hash: {}", e));
            return;
        }
    };

    messages::msg(format!("  Using commit: {}", commit_hash));

    // Step 4: Setup build environment for install
    let branch_to_use = branch.or_else(|| {
        if !package_def.branch.is_empty() {
            Some(package_def.branch.clone())
        } else {
            None
        }
    });

    let build_env = BuildEnvironment::new(
        package.to_string(),
        commit_hash.clone(),
        branch_to_use,
        config.local_storage_path(),
        install_prefix.clone(),
    );

    // Step 5: Check that artifacts exist
    if !build_env.artifacts_exist() {
        messages::failure(format!(
            "Artifacts not found for commit {}",
            commit_hash
        ));
        messages::msg(format!("Expected artifacts at: {}", build_env.artifacts_dir.display()));
        messages::msg("Please ensure all prerequisites have been executed successfully.");
        return;
    }

    messages::msg(format!("Artifacts found at: {}", build_env.artifacts_dir.display()));

    // Step 6: Execute install command
    messages::msg("Executing install stage...");
    
    if let Err(e) = execute_install_stage(&build_env, install_stage, verbose, noconfirm) {
        messages::failure(format!("Install failed: {}", e));
        return;
    }

    messages::success(format!("Package '{}' installed successfully to {}!", package, install_prefix.display()));
}

/// Execute the build prerequisite by calling the build command
fn execute_build_prerequisite(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    branch: Option<String>,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    // Call the build command handler
    // Use default parameters: container build (from config), not local, not chroot, not forced
    handle_build(
        config,
        system_info,
        package,
        branch,
        None, // tag
        None, // container (use config default)
        false, // local
        false, // chroot
        false, // force
        verbose,
        noconfirm,
    );
    
    Ok(())
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

/// Execute the install stage
fn execute_install_stage(
    build_env: &BuildEnvironment,
    install_stage: &crate::utilities::PackageStage,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    let install_command = match &install_stage.command {
        Some(cmd) => cmd,
        None => {
            return Err("Install stage does not have a command defined".to_string());
        }
    };

    if verbose {
        logging::debug(format!("Install command: {}", install_command));
    }

    // Prepare environment variables
    let env_vars = build_env.get_env_vars();

    if verbose {
        logging::debug("Install environment variables:");
        for (key, value) in &env_vars {
            logging::debug(format!("  {}={}", key, value));
        }
    }

    // Execute install command
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(install_command)
        .current_dir(&build_env.source_dir);

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    messages::info(format!("Install command: sh -c '{}'", install_command));
    messages::info(format!("Install prefix: {}", build_env.install_prefix.display()));

    // Ask for confirmation unless noconfirm is set
    if !noconfirm {
        if !messages::confirm("Proceed with installation?") {
            return Err("Installation cancelled by user".to_string());
        }
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute install command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Install command failed:\n{}", stderr));
    }

    if verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            logging::debug(format!("Install output:\n{}", stdout));
        }
    }

    Ok(())
}

/// Check if the current process is running as root
fn is_running_as_root() -> bool {
    // Check if effective UID is 0 (root)
    Uid::effective().is_root()
}
