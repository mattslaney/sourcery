use crate::config::Config;
use crate::logging;
use crate::system::SystemInfo;
use crate::utilities::{
    load_package, BuildEnvironment, ContainerRuntime, SourceryImageBuilder, GitRepo,
    get_build_volume_mounts, format_env_for_container,
};
use crate::{green, red};
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
    confirm: bool,
    verbose: bool,
    noconfirm: bool,
) {
    logging::msg(format!("Building package: {}", package));
    logging::msg(format!(
        "  Target system: {} {} ({})",
        system_info.distro_name, system_info.distro_version_id, system_info.arch
    ));

    if let Some(ref branch) = branch {
        logging::msg(format!("  From branch: {}", branch));
    }
    if let Some(ref tag) = tag {
        logging::msg(format!("  From tag: {}", tag));
    }

    // Determine build environment
    let use_container = if container.is_some() {
        true
    } else if local {
        false
    } else if chroot {
        logging::msg(red!("Chroot builds are not yet implemented"));
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

    logging::msg(format!("  Build environment: {}", environment));

    // Step 1: Load package definition
    logging::msg("Loading package definition...");
    
    let package_def = match load_package_definition(config, package, &system_info.distro_id, verbose) {
        Ok(pkg) => pkg,
        Err(e) => {
            logging::msg(red!("Failed to load package: {}", e));
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
            logging::msg(red!("Package '{}' does not have a build stage defined", package));
            return;
        }
    };

    // Step 2: Clone/update source repository
    logging::msg("Setting up source repository...");
    
    let source_dir = config.local_storage_path().join("source").join(package);
    let branch_to_use = branch.or_else(|| {
        if !package_def.branch.is_empty() {
            Some(package_def.branch.clone())
        } else {
            None
        }
    });

    if package_def.repo.is_empty() {
        logging::msg(red!("Package '{}' does not have a repository URL defined", package));
        return;
    }

    let repo_url = &package_def.repo[0]; // Use first repository URL

    let git_repo = match setup_source_repository(repo_url, &source_dir, branch_to_use.as_deref(), tag.as_deref(), verbose) {
        Ok(repo) => repo,
        Err(e) => {
            logging::msg(red!("Failed to setup source repository: {}", e));
            return;
        }
    };

    // Get commit hash
    let commit_hash = match git_repo.get_short_commit_hash() {
        Ok(hash) => hash,
        Err(e) => {
            logging::msg(red!("Failed to get commit hash: {}", e));
            return;
        }
    };

    logging::msg(format!("  Source ready at commit: {}", commit_hash));

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
        logging::msg(red!("Failed to setup build directories: {}", e));
        return;
    }

    // Check if artifacts already exist
    if build_env.artifacts_exist() && !noconfirm {
        logging::msg(format!("Artifacts already exist for commit {}", commit_hash));
        if !confirm {
            logging::msg("Skipping build. Use --confirm to rebuild.");
            return;
        }
    }

    // Step 4: Execute build
    if use_container {
        if let Err(e) = build_with_container(&build_env, build_stage, system_info, verbose) {
            logging::msg(red!("Build failed: {}", e));
            return;
        }
    } else {
        if let Err(e) = build_local(&build_env, build_stage, verbose) {
            logging::msg(red!("Build failed: {}", e));
            return;
        }
    }

    logging::msg(green!("Build completed successfully!"));
    logging::msg(format!("Artifacts available at: {}", build_env.artifacts_dir.display()));
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
) -> Result<(), String> {
    logging::msg("Building in container environment...");

    // Detect container runtime
    let runtime = ContainerRuntime::detect()
        .map_err(|e| format!("Container runtime not found: {}", e))?;

    logging::msg(format!("  Using container runtime: {}", runtime.command()));

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

    logging::msg(format!("  Building base image for {} {}...", system_info.distro_id, version.unwrap_or("latest")));
    
    let image_tag = image_builder.build_base_image(&system_info.distro_id, version)?;

    logging::msg(format!("  Using image: {}", image_tag));

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

    if verbose {
        logging::debug(format!("Build script:\n{}", build_script));
    }

    logging::msg("  Executing build command...");

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

    Ok(())
}

/// Build package locally
fn build_local(
    build_env: &BuildEnvironment,
    build_stage: &crate::utilities::PackageStage,
    verbose: bool,
) -> Result<(), String> {
    logging::msg("Building locally...");

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

    logging::msg("  Executing build command...");

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

    Ok(())
}
