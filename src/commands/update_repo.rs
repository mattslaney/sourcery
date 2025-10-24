use crate::config::Config;
use crate::system::SystemInfo;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::Path;
use std::process::Command;

pub fn handle_update_repo(config: &Config, _system_info: &SystemInfo) {
    println!("Updating package repositories...");

    // Get the repositories directory path
    let repositories_dir = config.local_storage_path().join("repositories");

    // Ensure the repositories directory exists
    if let Err(e) = ensure_directory_exists(&repositories_dir) {
        eprintln!("Error creating repositories directory: {}", e);
        std::process::exit(1);
    }

    // Process each repository in the config
    for (name, repo_info) in &config.repositories {
        println!("\nProcessing repository '{}'...", name);

        if repo_info.is_path_based() {
            // Handle path-based repository
            let source_path = repo_info.expanded_path().unwrap();
            println!("  Path: {}", source_path.display());

            if !source_path.exists() {
                eprintln!("  ⚠ Warning: Path does not exist: {}", source_path.display());
                continue;
            }

            if !source_path.is_dir() {
                eprintln!("  ⚠ Warning: Path is not a directory: {}", source_path.display());
                continue;
            }

            // Use the repository key as the symlink name
            let link_path = repositories_dir.join(name);

            if link_path.exists() || link_path.is_symlink() {
                // Check if it's a symlink and if it points to the right place
                if link_path.is_symlink() {
                    match fs::read_link(&link_path) {
                        Ok(target) => {
                            if target == source_path {
                                println!("  ✓ Symlink already exists and is correct");
                                continue;
                            } else {
                                println!("  Updating symlink (target changed)...");
                                if let Err(e) = fs::remove_file(&link_path) {
                                    eprintln!("  Error removing old symlink: {}", e);
                                    continue;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("  Error reading symlink: {}", e);
                            continue;
                        }
                    }
                } else {
                    eprintln!("  ⚠ Warning: Path exists but is not a symlink: {}", link_path.display());
                    eprintln!("  Please remove it manually to use a path-based repository");
                    continue;
                }
            }

            // Create symlink
            if let Err(e) = unix_fs::symlink(&source_path, &link_path) {
                eprintln!("  Error creating symlink: {}", e);
            } else {
                println!("  ✓ Symlink created successfully");
            }
        } else {
            // Handle URL-based repository
            let url = repo_info.url.as_ref().unwrap();
            let branch = repo_info.branch.as_ref().unwrap();
            
            println!("  URL: {}", url);
            println!("  Branch: {}", branch);

            // Use the repository key as the directory name
            let repo_path = repositories_dir.join(name);

            if repo_path.exists() {
                // Repository exists, update it
                println!("  Repository exists at: {}", repo_path.display());
                if let Err(e) = update_repository(&repo_path, branch) {
                    eprintln!("  Error updating repository: {}", e);
                } else {
                    println!("  ✓ Repository updated successfully");
                }
            } else {
                // Repository doesn't exist, clone it
                println!("  Repository not found, cloning...");
                if let Err(e) = clone_repository(url, &repositories_dir, name, branch) {
                    eprintln!("  Error cloning repository: {}", e);
                } else {
                    println!("  ✓ Repository cloned successfully");
                }
            }
        }
    }

    println!("\nRepository update complete!");
}

/// Ensure a directory exists, creating it if necessary
fn ensure_directory_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        println!("Creating directory: {}", path.display());
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Clone a git repository with a specific directory name and branch
fn clone_repository(
    url: &str,
    target_dir: &Path,
    repo_name: &str,
    branch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .arg("clone")
        .arg("--branch")
        .arg(branch)
        .arg(url)
        .arg(repo_name)
        .current_dir(target_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git clone failed: {}", stderr).into());
    }

    Ok(())
}

/// Update an existing git repository on a specific branch
fn update_repository(repo_path: &Path, branch: &str) -> Result<(), Box<dyn std::error::Error>> {
    // First, fetch all updates
    let output = Command::new("git")
        .arg("fetch")
        .arg("--all")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git fetch failed: {}", stderr).into());
    }

    // Checkout the desired branch
    let output = Command::new("git")
        .arg("checkout")
        .arg(branch)
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git checkout failed: {}", stderr).into());
    }

    // Pull the latest changes for the branch
    let output = Command::new("git")
        .arg("pull")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git pull failed: {}", stderr).into());
    }

    Ok(())
}
