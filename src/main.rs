use nix::unistd::Uid;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Config {
    local_storage: String,
    install: InstallConfig,
    build: BuildConfig,
    repositories: RepositoriesConfig,
}

#[derive(Debug, Deserialize)]
struct InstallConfig {
    user_path: String,
    system_path: String,
}

#[derive(Debug, Deserialize)]
struct BuildConfig {
    default_environment: String,
}

#[derive(Debug, Deserialize)]
struct RepositoriesConfig {
    main: String,
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            let mut home_path = PathBuf::from(home);
            home_path.push(&path[2..]);
            return home_path;
        }
    }
    PathBuf::from(path)
}

fn find_config_file(locations: &[&str]) -> Option<PathBuf> {
    for location in locations {
        let path = expand_tilde(location);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn load_config(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

fn main() {
    if !Uid::effective().is_root() {
        eprintln!("Error: This program must be run as root");
        std::process::exit(1);
    }

    let config_locations = vec![
        "config/sourcery.toml",
        "~/.config/sourcery/sourcery.toml",
        "/etc/sourcery/sourcery.toml",
    ];

    let config_path = match find_config_file(&config_locations) {
        Some(path) => path,
        None => {
            eprintln!("Error: No config file found in the following locations:");
            for location in &config_locations {
                eprintln!("  - {}", location);
            }
            std::process::exit(1);
        }
    };

    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading config from {}: {}", config_path.display(), e);
            std::process::exit(1);
        }
    };

    println!("Successfully loaded config from: {}", config_path.display());
    println!("Config: {:#?}", config);
}
