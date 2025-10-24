use crate::config::Config;
use crate::logging;
use crate::messages;
use crate::system::SystemInfo;
use std::fs;
use std::io::{self, Write};

pub fn handle_clean(config: &Config, _system_info: &SystemInfo, noconfirm: bool) {
    messages::msg("Cleaning sources and artifacts...");
    
    let base_dir = config.local_storage_path();
    let source_dir = base_dir.join("source");
    let artifacts_dir = base_dir.join("artifacts");
    
    // Track what we're about to delete
    let mut sources_to_clean = Vec::new();
    let mut artifacts_to_clean = Vec::new();
    
    // List sources
    if source_dir.exists() {
        match fs::read_dir(&source_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        sources_to_clean.push(file_name);
                    }
                }
            }
            Err(e) => {
                logging::warning(format!("Failed to read source directory: {}", e));
            }
        }
    }
    
    // List artifacts organized by package
    if artifacts_dir.exists() {
        match fs::read_dir(&artifacts_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Ok(package_name) = entry.file_name().into_string() {
                        let package_dir = entry.path();
                        let mut package_artifacts = Vec::new();
                        
                        // Read commit hash directories
                        if let Ok(commit_entries) = fs::read_dir(&package_dir) {
                            for commit_entry in commit_entries.flatten() {
                                if let Ok(commit_hash) = commit_entry.file_name().into_string() {
                                    let commit_dir = commit_entry.path();
                                    let mut artifact_files = Vec::new();
                                    
                                    // Read artifact files
                                    if let Ok(artifact_entries) = fs::read_dir(&commit_dir) {
                                        for artifact_entry in artifact_entries.flatten() {
                                            if let Ok(artifact_name) = artifact_entry.file_name().into_string() {
                                                artifact_files.push(artifact_name);
                                            }
                                        }
                                    }
                                    
                                    package_artifacts.push((commit_hash, artifact_files));
                                }
                            }
                        }
                        
                        artifacts_to_clean.push((package_name, package_artifacts));
                    }
                }
            }
            Err(e) => {
                logging::warning(format!("Failed to read artifacts directory: {}", e));
            }
        }
    }
    
    // Display what will be cleaned
    if sources_to_clean.is_empty() && artifacts_to_clean.is_empty() {
        messages::msg("Nothing to clean.");
        return;
    }
    
    // List repositories being cleaned
    if !sources_to_clean.is_empty() {
        println!("\n{}", style!("bold,cyan", "Repositories:"));
        for source in &sources_to_clean {
            println!("  • {}", source);
        }
    }
    
    // List artifacts being cleaned
    if !artifacts_to_clean.is_empty() {
        println!("\n{}", style!("bold,cyan", "Artifacts:"));
        for (package_name, commits) in &artifacts_to_clean {
            println!("  • {}", style!("bold", "{}", package_name));
            for (commit_hash, artifacts) in commits {
                println!("    ├─ {}", style!("dim", "{}", commit_hash));
                for artifact in artifacts {
                    println!("    │  └─ {}", artifact);
                }
            }
        }
    }
    
    println!();
    
    // Ask for confirmation unless --noconfirm is specified
    if !noconfirm {
        print!("Do you want to proceed with cleaning? [y/N] ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            logging::error("Failed to read user input");
            return;
        }
        
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            messages::msg("Clean cancelled.");
            return;
        }
        println!();
    }
    
    // Perform the cleanup
    let mut errors = Vec::new();
    
    // Clean sources
    if source_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&source_dir) {
            errors.push(format!("Failed to remove source directory: {}", e));
        } else {
            messages::msg(format!("✓ Removed source directory: {}", source_dir.display()));
        }
    }
    
    // Clean artifacts
    if artifacts_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&artifacts_dir) {
            errors.push(format!("Failed to remove artifacts directory: {}", e));
        } else {
            messages::msg(format!("✓ Removed artifacts directory: {}", artifacts_dir.display()));
        }
    }
    
    // Report any errors
    if !errors.is_empty() {
        println!();
        for error in errors {
            logging::error(error);
        }
    } else {
        println!();
        messages::msg("Clean completed successfully!");
    }
}
