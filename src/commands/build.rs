use crate::config::Config;
use crate::logging;
use crate::messages;
use crate::security;
use crate::system::SystemInfo;
use crate::utilities::{
    load_package, BuildEnvironment, ContainerRuntime, ContainerRuntimeType, 
    SourceryImageBuilder, GitRepo, get_build_volume_mounts, format_env_for_container,
};
use crate::utils;
use std::fs;

#[allow(clippy::too_many_arguments)]
pub fn handle_build(
    config: &Config,
    system_info: &SystemInfo,
    package: &str,
    branch: Option<String>,
    tag: Option<String>,
    container: Option<Option<String>>,
    local: bool,
    chroot: bool,
    force: bool,
    verbose: bool,
    noconfirm: bool,
    allow_dangerous: bool,
) {
    // Build operations never need root - drop privileges if running with sudo
    utils::ensure_not_root("build", verbose);
    
    messages::msg(format!("Building package: {}", package));
    messages::msg(format!(
        "  Target system: {} {} ({})",
        system_info.distro_name, system_info.distro_version_id, system_info.arch
    ));

    if let Some(ref branch) = branch {
        messages::msg(format!("  From branch: {}", branch));
    }
    if let Some(ref tag) = tag {
        messages::msg(format!("  From tag: {}", tag));
    }

    // Determine build environment
    let use_container = if container.is_some() {
        true
    } else if local {
        false
    } else if chroot {
        messages::failure("Chroot builds are not yet implemented");
        return;
    } else {
        // Use default from config
        config.build.default_environment == "container"
    };

    let environment = if use_container {
        "container"
    } else {
        "local"
    };

    messages::msg(format!("  Build environment: {}", environment));

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

    // Check if package has build stage
    let build_stage = match &package_def.build {
        Some(stage) => stage,
        None => {
            messages::failure(format!("Package '{}' does not have a build stage defined", package));
            return;
        }
    };

    // Step 2: Clone/update source repository
    messages::msg("Setting up source repository...");
    
    let source_dir = config.local_storage_path().join("source").join(package);
    let branch_to_use = branch.or_else(|| {
        if !package_def.branch.is_empty() {
            Some(package_def.branch.clone())
        } else {
            None
        }
    });

    if package_def.repo.is_empty() {
        messages::failure(format!("Package '{}' does not have a repository URL defined", package));
        return;
    }

    let repo_url = &package_def.repo[0]; // Use first repository URL

    let git_repo = match setup_source_repository(repo_url, &source_dir, branch_to_use.as_deref(), tag.as_deref(), verbose) {
        Ok(repo) => repo,
        Err(e) => {
            messages::failure(format!("Failed to setup source repository: {}", e));
            return;
        }
    };

    // Get commit hash
    let commit_hash = match git_repo.get_short_commit_hash() {
        Ok(hash) => hash,
        Err(e) => {
            messages::failure(format!("Failed to get commit hash: {}", e));
            return;
        }
    };

    messages::msg(format!("  Source ready at commit: {}", commit_hash));

    // Step 3: Setup build environment
    let install_prefix = config.system_install_path();
    let build_env = BuildEnvironment::new(
        package.to_string(),
        commit_hash.clone(),
        branch_to_use,
        config.local_storage_path(),
        install_prefix,
    );

    if let Err(e) = build_env.setup_directories() {
        messages::failure(format!("Failed to setup build directories: {}", e));
        return;
    }

    // Check if artifacts already exist
    if build_env.artifacts_exist() && !force {
        messages::msg(format!("Artifacts already exist for commit {}", commit_hash));
        messages::msg("Skipping build. Use --force to rebuild.");
        return;
    }

    // Step 4: Execute build
    if use_container {
        if let Err(e) = build_with_container(&build_env, build_stage, system_info, verbose, noconfirm) {
            messages::failure(format!("Build failed: {}", e));
            return;
        }
    } else {
        if let Err(e) = build_local(&build_env, build_stage, verbose, noconfirm, allow_dangerous) {
            messages::failure(format!("Build failed: {}", e));
            return;
        }
    }

    messages::success("Build completed successfully!");
    messages::msg(format!("Artifacts available at: {}", build_env.artifacts_dir.display()));
}

/// Load package definition with system-specific overrides
fn load_package_definition(
    config: &Config,
    package_name: &str,
    distro_id: &str,
    verbose: bool,
) -> Result<crate::utilities::Package, String> {
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

/// Setup source repository (clone or update)
fn setup_source_repository(
    repo_url: &str,
    dest_path: &std::path::Path,
    branch: Option<&str>,
    tag: Option<&str>,
    verbose: bool,
) -> Result<GitRepo, String> {
    let repo = if dest_path.exists() {
        if verbose {
            logging::debug("Repository already exists, updating...");
        }
        let repo = GitRepo::new(dest_path);
        
        // Clean the repository to remove any build artifacts and uncommitted changes
        if verbose {
            logging::debug("Cleaning repository to pristine state...");
        }
        repo.clean()?;
        
        // Fetch latest changes
        repo.fetch()?;
        repo
    } else {
        if verbose {
            logging::debug(format!("Cloning repository from {}...", repo_url));
        }
        GitRepo::clone(repo_url, dest_path, branch)?
    };

    // Checkout specific tag or branch if requested
    if let Some(tag) = tag {
        if verbose {
            logging::debug(format!("Checking out tag: {}", tag));
        }
        repo.checkout(tag)?;
    } else if let Some(branch) = branch {
        if verbose {
            logging::debug(format!("Checking out branch: {}", branch));
        }
        repo.checkout(branch)?;
    }

    Ok(repo)
}

/// Build package in a container
fn build_with_container(
    build_env: &BuildEnvironment,
    build_stage: &crate::utilities::PackageStage,
    system_info: &SystemInfo,
    verbose: bool,
    noconfirm: bool,
) -> Result<(), String> {
    messages::msg("Building in container environment...");

    // Detect container runtime
    let runtime = ContainerRuntime::detect()
        .map_err(|e| format!("Container runtime not found: {}", e))?;

    messages::msg(format!("  Using container runtime: {}", runtime.command()));

    // Build base image if needed
    let containerfiles_dir = std::path::PathBuf::from("/etc/sourcery/containers");
    
    // Fallback to local containers directory if /etc/sourcery doesn't exist
    let containerfiles_dir = if containerfiles_dir.exists() {
        containerfiles_dir
    } else {
        if verbose {
            logging::debug("Using local containers directory");
        }
        std::path::PathBuf::from("containers")
    };

    let image_builder = SourceryImageBuilder::new(runtime.clone(), containerfiles_dir);

    let version = if !system_info.distro_version_id.is_empty() {
        Some(system_info.distro_version_id.as_str())
    } else {
        None
    };

    messages::msg(format!("  Building base image for {} {}...", system_info.distro_id, version.unwrap_or("latest")));
    
    let image_tag = image_builder.build_base_image(&system_info.distro_id, version)?;

    messages::msg(format!("  Using image: {}", image_tag));

    // Prepare environment variables
    let env_vars = build_env.get_env_vars();
    let env_vec = format_env_for_container(&env_vars);

    // Prepare volume mounts
    let volumes = get_build_volume_mounts(build_env);

    // Get build command
    let build_command = match &build_stage.command {
        Some(cmd) => cmd,
        None => {
            return Err("Build stage does not have a command defined".to_string());
        }
    };

    // Install build dependencies if specified
    let deps_install = if let Some(deps) = &build_stage.dependencies {
        if !deps.is_empty() {
            // Detect package manager based on distro
            let pkg_manager = match system_info.distro_id.as_str() {
                "ubuntu" | "debian" => {
                    format!("apt-get update && apt-get install -y {}", deps.join(" "))
                },
                "fedora" | "centos" | "rhel" => {
                    format!("dnf install -y {}", deps.join(" "))
                },
                "arch" => {
                    format!("pacman -Sy --noconfirm {}", deps.join(" "))
                },
                "opensuse" => {
                    format!("zypper install -y {}", deps.join(" "))
                },
                "alpine" => {
                    format!("apk add {}", deps.join(" "))
                },
                _ => {
                    format!("apt-get update && apt-get install -y {}", deps.join(" "))
                }
            };
            format!("echo 'Installing build dependencies...'\n{}\n", pkg_manager)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Prepare the full build script
    let build_script = format!(
        r#"#!/bin/sh
set -e
{}cd /usr/src/package
{}
"#,
        deps_install,
        build_command
    );
    messages::msg("  Executing build command...");

    // Show the container execution details
    messages::info(format!("Container: {} run --rm {} {}", 
        runtime.command(), 
        image_tag,
        if runtime.runtime_type() == ContainerRuntimeType::Podman {
            "--userns=keep-id"
        } else {
            ""
        }
    ));
    
    // Show the build script that will be executed inside the container
    messages::info("Commands to execute in container:");
    for line in build_script.lines().skip(2) { // Skip shebang and set -e
        if !line.is_empty() {
            messages::info(format!("  {}", line));
        }
    }

    if verbose {
        messages::info(format!("Full build script:\n{}", build_script));
    }

    // Ask for confirmation unless noconfirm is set
    if !noconfirm {
        if !messages::confirm("Proceed with build?") {
            return Err("Build cancelled by user".to_string());
        }
    }

    // Run the build
    let output = runtime.run(&image_tag, &build_script, &volumes, Some(&env_vec))
        .map_err(|e| format!("Failed to run container: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Build command failed:\n{}", stderr));
    }

    if verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            logging::debug(format!("Build output:\n{}", stdout));
        }
    }

    // Save build log
    let log_content = format!(
        "Build log for {} at commit {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}\n",
        build_env.package_name,
        build_env.commit_hash,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if let Err(e) = fs::write(build_env.get_log_file_path(), log_content) {
        logging::error(format!("Warning: Failed to write build log: {}", e));
    }

    // Copy artifacts from source to artifacts directory
    if let Err(e) = copy_artifacts(build_env, build_stage, verbose) {
        return Err(format!("Failed to copy artifacts: {}", e));
    }

    Ok(())
}

/// Build package locally
fn build_local(
    build_env: &BuildEnvironment,
    build_stage: &crate::utilities::PackageStage,
    verbose: bool,
    noconfirm: bool,
    allow_dangerous: bool,
) -> Result<(), String> {
    messages::msg("Building locally...");

    // Get build command
    let build_command = match &build_stage.command {
        Some(cmd) => cmd,
        None => {
            return Err("Build stage does not have a command defined".to_string());
        }
    };

    if verbose {
        logging::debug(format!("Build command: {}", build_command));
    }

    // Perform security analysis on the command
    let security_analysis = security::analyze_command(build_command);
    
    match security_analysis.level {
        security::SecurityLevel::Blocked => {
            messages::failure("❌ This command contains forbidden operations that will never be executed:");
            for reason in &security_analysis.reasons {
                messages::failure(format!("   - {}", reason));
            }
            messages::blank();
            messages::failure("This package cannot be built for safety reasons.");
            return Err("Command blocked for security reasons".to_string());
        }
        security::SecurityLevel::Caution => {
            if !allow_dangerous {
                messages::caution("⚠️ Some actions in this command have been identified as potentially dangerous.");
                messages::caution("Dangerous patterns detected:");
                for reason in &security_analysis.reasons {
                    messages::caution(format!("   - {}", reason));
                }
                messages::blank();
                messages::caution("Please carefully review the commands before execution:");
                messages::blank();
                for line in build_command.lines() {
                    messages::info(format!("   {}", line));
                }
                messages::blank();
            }
        }
        security::SecurityLevel::Safe => {
            // No additional warnings needed
        }
    }

    // Prepare environment
    let env_vars = build_env.get_env_vars();

    // Execute build command
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(build_command)
        .current_dir(&build_env.source_dir);

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    messages::msg("  Executing build command...");
    
    // Show the command that will be executed
    messages::info(format!("Build command: sh -c '{}'", build_command));
    messages::info(format!("Working directory: {}", build_env.source_dir.display()));

    // Ask for confirmation unless noconfirm is set
    if !noconfirm {
        if !messages::confirm("Proceed with build?") {
            return Err("Build cancelled by user".to_string());
        }
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to execute build command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Build command failed:\n{}", stderr));
    }

    if verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            logging::debug(format!("Build output:\n{}", stdout));
        }
    }

    // Save build log
    let log_content = format!(
        "Build log for {} at commit {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}\n",
        build_env.package_name,
        build_env.commit_hash,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if let Err(e) = fs::write(build_env.get_log_file_path(), log_content) {
        logging::error(format!("Warning: Failed to write build log: {}", e));
    }

    // Copy artifacts from source to artifacts directory
    if let Err(e) = copy_artifacts(build_env, build_stage, verbose) {
        return Err(format!("Failed to copy artifacts: {}", e));
    }

    Ok(())
}

/// Copy artifacts from source directory to artifacts directory
/// Preserves the full directory structure as specified in the artifacts list
fn copy_artifacts(
    build_env: &BuildEnvironment,
    build_stage: &crate::utilities::PackageStage,
    verbose: bool,
) -> Result<(), String> {
    // Check if there are artifacts to copy
    let artifacts = match &build_stage.artifacts {
        Some(artifacts) if !artifacts.is_empty() => artifacts,
        _ => {
            if verbose {
                logging::debug("No artifacts specified to copy");
            }
            return Ok(());
        }
    };

    messages::msg(format!("Copying {} artifact(s)...", artifacts.len()));

    for artifact_path in artifacts {
        let source_path = build_env.source_dir.join(artifact_path);
        
        if !source_path.exists() {
            return Err(format!(
                "Artifact '{}' does not exist at {}",
                artifact_path,
                source_path.display()
            ));
        }

        // Preserve the full path structure in the artifacts directory
        let dest_path = build_env.artifacts_dir.join(artifact_path);

        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create artifact directory '{}': {}",
                    parent.display(), e
                )
            })?;
        }

        if verbose {
            logging::debug(format!(
                "Copying {} -> {}",
                source_path.display(),
                dest_path.display()
            ));
        }

        // Copy the file or directory
        if source_path.is_file() {
            fs::copy(&source_path, &dest_path).map_err(|e| {
                format!(
                    "Failed to copy artifact '{}': {}",
                    artifact_path, e
                )
            })?;
        } else if source_path.is_dir() {
            // For directories, use a recursive copy
            copy_dir_recursive(&source_path, &dest_path)?;
        } else {
            return Err(format!(
                "Artifact '{}' is neither a file nor a directory",
                artifact_path
            ));
        }

        messages::msg(format!("  ✓ Copied {}", artifact_path));
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory '{}': {}", dst.display(), e))?;

    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory '{}': {}", src.display(), e))?
    {
        let entry = entry
            .map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path).map_err(|e| {
                format!(
                    "Failed to copy file '{}' to '{}': {}",
                    path.display(),
                    dest_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}
