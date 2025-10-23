use crate::config::Config;
use crate::system::SystemInfo;
use std::fs;
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
        println!("  URL: {}", repo_info.url);
        println!("  Branch: {}", repo_info.branch);

        // Use the repository key as the directory name
        let repo_path = repositories_dir.join(name);

        if repo_path.exists() {
            // Repository exists, update it
            println!("  Repository exists at: {}", repo_path.display());
            if let Err(e) = update_repository(&repo_path, &repo_info.branch) {
                eprintln!("  Error updating repository: {}", e);
            } else {
                println!("  ✓ Repository updated successfully");
            }
        } else {
            // Repository doesn't exist, clone it
            println!("  Repository not found, cloning...");
            if let Err(e) =
                clone_repository(&repo_info.url, &repositories_dir, name, &repo_info.branch)
            {
                eprintln!("  Error cloning repository: {}", e);
            } else {
                println!("  ✓ Repository cloned successfully");
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
