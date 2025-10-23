use clap::{Parser, Subcommand};
use nix::unistd::Uid;
use serde::Deserialize;
use std::collections::HashMap;
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

#[derive(Debug, Clone)]
struct SystemInfo {
    /// System architecture (e.g., "x86_64", "aarch64")
    arch: String,
    /// Distribution ID (e.g., "ubuntu", "fedora", "debian")
    distro_id: String,
    /// Distribution name (e.g., "Ubuntu", "Fedora")
    distro_name: String,
    /// Distribution version (e.g., "22.04", "38")
    distro_version: String,
    /// Distribution version ID (e.g., "22.04", "38")
    distro_version_id: String,
    /// Distribution version codename (e.g., "jammy", "bookworm")
    distro_version_codename: Option<String>,
}

impl SystemInfo {
    fn detect() -> Result<Self, Box<dyn std::error::Error>> {
        let arch = std::env::consts::ARCH.to_string();
        let os_release = Self::parse_os_release()?;
        
        let distro_id = os_release.get("ID")
            .ok_or("Missing ID in os-release")?
            .trim_matches('"')
            .to_string();
        
        let distro_name = os_release.get("NAME")
            .ok_or("Missing NAME in os-release")?
            .trim_matches('"')
            .to_string();
        
        let distro_version = os_release.get("VERSION")
            .unwrap_or(&String::from(""))
            .trim_matches('"')
            .to_string();
        
        let distro_version_id = os_release.get("VERSION_ID")
            .unwrap_or(&String::from(""))
            .trim_matches('"')
            .to_string();
        
        let distro_version_codename = os_release.get("VERSION_CODENAME")
            .map(|s| s.trim_matches('"').to_string());
        
        Ok(SystemInfo {
            arch,
            distro_id,
            distro_name,
            distro_version,
            distro_version_id,
            distro_version_codename,
        })
    }
    
    fn parse_os_release() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        // Try /etc/os-release first (standard location)
        let os_release_paths = vec![
            "/etc/os-release",
            "/usr/lib/os-release",
        ];
        
        let mut content = String::new();
        for path in os_release_paths {
            if let Ok(c) = fs::read_to_string(path) {
                content = c;
                break;
            }
        }
        
        if content.is_empty() {
            // Try legacy release files
            content = Self::try_legacy_release_files()?;
        }
        
        let mut map = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            }
        }
        
        if map.is_empty() {
            return Err("No os-release information found".into());
        }
        
        Ok(map)
    }
    
    fn try_legacy_release_files() -> Result<String, Box<dyn std::error::Error>> {
        // Try various legacy release files
        let legacy_files = vec![
            "/etc/lsb-release",
            "/etc/debian_version",
            "/etc/redhat-release",
            "/etc/fedora-release",
            "/etc/centos-release",
            "/etc/arch-release",
            "/etc/gentoo-release",
            "/etc/slackware-version",
        ];
        
        for path in legacy_files {
            if let Ok(content) = fs::read_to_string(path) {
                // Try to extract basic info from the content
                if path == "/etc/lsb-release" {
                    return Ok(content);
                } else if path == "/etc/debian_version" {
                    return Ok(format!("ID=debian\nNAME=\"Debian\"\nVERSION_ID=\"{}\"", content.trim()));
                } else {
                    // For other files, try to parse the content
                    return Ok(Self::parse_legacy_content(&content, path));
                }
            }
        }
        
        Err("No release files found".into())
    }
    
    fn parse_legacy_content(content: &str, path: &str) -> String {
        let content = content.trim();
        
        if path.contains("redhat") {
            format!("ID=rhel\nNAME=\"Red Hat Enterprise Linux\"\nVERSION=\"{}\"", content)
        } else if path.contains("fedora") {
            format!("ID=fedora\nNAME=\"Fedora\"\nVERSION=\"{}\"", content)
        } else if path.contains("centos") {
            format!("ID=centos\nNAME=\"CentOS\"\nVERSION=\"{}\"", content)
        } else if path.contains("arch") {
            format!("ID=arch\nNAME=\"Arch Linux\"\nVERSION=\"rolling\"")
        } else if path.contains("gentoo") {
            format!("ID=gentoo\nNAME=\"Gentoo\"\nVERSION=\"{}\"", content)
        } else if path.contains("slackware") {
            format!("ID=slackware\nNAME=\"Slackware\"\nVERSION=\"{}\"", content)
        } else {
            format!("ID=unknown\nNAME=\"Unknown\"\nVERSION=\"{}\"", content)
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "sourcery")]
#[command(about = "A package manager for maintaining system packages from source", long_about = None)]
struct Cli {
    /// Update the package repositories
    #[arg(long)]
    update: bool,

    /// Clean out old sources and outputs
    #[arg(long)]
    clean: bool,

    /// Check health of scry (dependencies, container runtime, etc)
    #[arg(long)]
    health: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List packages
    List {
        /// List packages installed with scry
        #[arg(long)]
        installed: bool,

        /// List installed packages that are upgradable
        #[arg(long)]
        upgradable: bool,
    },

    /// Search for a package or collection
    Search {
        /// Package or collection name to search for
        query: String,

        /// Use fuzzy search (default)
        #[arg(long)]
        fuzzy: bool,

        /// Use exact search
        #[arg(long)]
        exact: bool,

        /// Search for a package
        #[arg(long)]
        package: bool,

        /// Search for a collection
        #[arg(long)]
        collection: bool,
    },

    /// Build a package
    Build {
        /// Package name to build
        package: String,

        /// Build from a specific branch
        #[arg(long)]
        branch: Option<String>,

        /// Build from a specific tag
        #[arg(long)]
        tag: Option<String>,

        /// Build in a container (optionally specify container image)
        #[arg(long)]
        container: Option<Option<String>>,

        /// Build on current system
        #[arg(long)]
        local: bool,

        /// Build in a chroot
        #[arg(long)]
        chroot: bool,

        /// Confirm before commands
        #[arg(long)]
        confirm: bool,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation
        #[arg(long)]
        noconfirm: bool,
    },

    /// Install a package
    Install {
        /// Package name to install
        package: String,

        /// Install from a specific branch
        #[arg(long)]
        branch: Option<String>,

        /// Install from a specific tag
        #[arg(long)]
        tag: Option<String>,

        /// Install for the current user
        #[arg(long)]
        user: bool,

        /// Install system-wide
        #[arg(long)]
        system: bool,

        /// Confirm before commands
        #[arg(long)]
        confirm: bool,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation
        #[arg(long)]
        noconfirm: bool,
    },

    /// Update a package
    Update {
        /// Package name to update
        package: String,

        /// Update from a specific branch
        #[arg(long)]
        branch: Option<String>,

        /// Update from a specific tag
        #[arg(long)]
        tag: Option<String>,

        /// Confirm before commands
        #[arg(long)]
        confirm: bool,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation
        #[arg(long)]
        noconfirm: bool,
    },

    /// Uninstall a package
    Uninstall {
        /// Package name to uninstall
        package: String,

        /// Confirm before commands
        #[arg(long)]
        confirm: bool,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation
        #[arg(long)]
        noconfirm: bool,
    },

    /// Uninstall and purge all package data
    Purge {
        /// Package name to purge
        package: String,

        /// Confirm before commands
        #[arg(long)]
        confirm: bool,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation
        #[arg(long)]
        noconfirm: bool,
    },
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
    // if !Uid::effective().is_root() {
    //     eprintln!("Error: This program must be run as root");
    //     std::process::exit(1);
    // }

    let cli = Cli::parse();

    // Detect system information
    let system_info = match SystemInfo::detect() {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Error detecting system information: {}", e);
            std::process::exit(1);
        }
    };

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

    // Handle top-level flags
    if cli.update {
        handle_update(&config, &system_info);
        return;
    }

    if cli.clean {
        handle_clean(&config, &system_info);
        return;
    }

    if cli.health {
        handle_health(&config, &system_info);
        return;
    }

    // Handle subcommands
    match cli.command {
        Some(Commands::List { installed, upgradable }) => {
            handle_list(&config, &system_info, installed, upgradable);
        }
        Some(Commands::Search { query, fuzzy, exact, package, collection }) => {
            handle_search(&config, &system_info, &query, fuzzy, exact, package, collection);
        }
        Some(Commands::Build { package, branch, tag, container, local, chroot, confirm, verbose, noconfirm }) => {
            handle_build(&config, &system_info, &package, branch, tag, container, local, chroot, confirm, verbose, noconfirm);
        }
        Some(Commands::Install { package, branch, tag, user, system, confirm, verbose, noconfirm }) => {
            handle_install(&config, &system_info, &package, branch, tag, user, system, confirm, verbose, noconfirm);
        }
        Some(Commands::Update { package, branch, tag, confirm, verbose, noconfirm }) => {
            handle_update_package(&config, &system_info, &package, branch, tag, confirm, verbose, noconfirm);
        }
        Some(Commands::Uninstall { package, confirm, verbose, noconfirm }) => {
            handle_uninstall(&config, &system_info, &package, confirm, verbose, noconfirm);
        }
        Some(Commands::Purge { package, confirm, verbose, noconfirm }) => {
            handle_purge(&config, &system_info, &package, confirm, verbose, noconfirm);
        }
        None => {
            eprintln!("No command specified. Use --help for usage information.");
            std::process::exit(1);
        }
    }
}

// Handler functions for each command
fn handle_update(_config: &Config, _system_info: &SystemInfo) {
    println!("Updating package repositories...");
    // TODO: Implement repository update logic
}

fn handle_clean(_config: &Config, _system_info: &SystemInfo) {
    println!("Cleaning old sources and outputs...");
    // TODO: Implement cleanup logic
}

fn handle_health(_config: &Config, system_info: &SystemInfo) {
    println!("System Health Check");
    println!("==================");
    println!();
    println!("System Information:");
    println!("  Architecture: {}", system_info.arch);
    println!("  Distribution: {} ({})", system_info.distro_name, system_info.distro_id);
    println!("  Version: {}", system_info.distro_version);
    if !system_info.distro_version_id.is_empty() {
        println!("  Version ID: {}", system_info.distro_version_id);
    }
    if let Some(codename) = &system_info.distro_version_codename {
        println!("  Codename: {}", codename);
    }
    println!();
    
    // TODO: Add checks for:
    // - Container runtime (podman/docker)
    // - Build dependencies
    // - Git
    // - Disk space
    // - Network connectivity
    println!("Additional health checks coming soon...");
}

fn handle_list(_config: &Config, _system_info: &SystemInfo, installed: bool, upgradable: bool) {
    if installed {
        println!("Listing installed packages...");
        // TODO: Implement list installed logic
    } else if upgradable {
        println!("Listing upgradable packages...");
        // TODO: Implement list upgradable logic
    } else {
        eprintln!("Error: Please specify --installed or --upgradable");
        std::process::exit(1);
    }
}

fn handle_search(_config: &Config, _system_info: &SystemInfo, query: &str, _fuzzy: bool, exact: bool, package: bool, collection: bool) {
    let search_type = if exact { "exact" } else { "fuzzy" };
    let target = if package {
        "package"
    } else if collection {
        "collection"
    } else {
        "package or collection"
    };
    
    println!("Searching for {} '{}' using {} search...", target, query, search_type);
    // TODO: Implement search logic
}

fn handle_build(
    _config: &Config,
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
    println!("Building package: {}", package);
    println!("  Target system: {} {} ({})", system_info.distro_name, system_info.distro_version_id, system_info.arch);
    
    if let Some(branch) = branch {
        println!("  From branch: {}", branch);
    }
    if let Some(tag) = tag {
        println!("  From tag: {}", tag);
    }
    
    let environment = if let Some(container_opt) = container {
        if let Some(image) = container_opt {
            format!("container ({})", image)
        } else {
            "container (default)".to_string()
        }
    } else if local {
        "local".to_string()
    } else if chroot {
        "chroot".to_string()
    } else {
        "default".to_string()
    };
    
    println!("  Environment: {}", environment);
    println!("  Confirm: {}, Verbose: {}, NoConfirm: {}", confirm, verbose, noconfirm);
    
    // TODO: Implement build logic
}

fn handle_install(
    _config: &Config,
    _system_info: &SystemInfo,
    package: &str,
    branch: Option<String>,
    tag: Option<String>,
    user: bool,
    system: bool,
    confirm: bool,
    verbose: bool,
    noconfirm: bool,
) {
    println!("Installing package: {}", package);
    
    if let Some(branch) = branch {
        println!("  From branch: {}", branch);
    }
    if let Some(tag) = tag {
        println!("  From tag: {}", tag);
    }
    
    let scope = if user {
        "user"
    } else if system {
        "system"
    } else {
        "default"
    };
    
    println!("  Scope: {}", scope);
    println!("  Confirm: {}, Verbose: {}, NoConfirm: {}", confirm, verbose, noconfirm);
    
    // TODO: Implement install logic
}

fn handle_update_package(
    _config: &Config,
    _system_info: &SystemInfo,
    package: &str,
    branch: Option<String>,
    tag: Option<String>,
    confirm: bool,
    verbose: bool,
    noconfirm: bool,
) {
    println!("Updating package: {}", package);
    
    if let Some(branch) = branch {
        println!("  From branch: {}", branch);
    }
    if let Some(tag) = tag {
        println!("  From tag: {}", tag);
    }
    
    println!("  Confirm: {}, Verbose: {}, NoConfirm: {}", confirm, verbose, noconfirm);
    
    // TODO: Implement update logic
}

fn handle_uninstall(
    _config: &Config,
    _system_info: &SystemInfo,
    package: &str,
    confirm: bool,
    verbose: bool,
    noconfirm: bool,
) {
    println!("Uninstalling package: {}", package);
    println!("  Confirm: {}, Verbose: {}, NoConfirm: {}", confirm, verbose, noconfirm);
    
    // TODO: Implement uninstall logic
}

fn handle_purge(
    _config: &Config,
    _system_info: &SystemInfo,
    package: &str,
    confirm: bool,
    verbose: bool,
    noconfirm: bool,
) {
    println!("Purging package: {}", package);
    println!("  Confirm: {}, Verbose: {}, NoConfirm: {}", confirm, verbose, noconfirm);
    
    // TODO: Implement purge logic
}
