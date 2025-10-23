mod cli;
mod commands;
mod config;
mod system;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use system::SystemInfo;

fn main() {
    let cli = Cli::parse();

    // Detect system information
    let system_info = match SystemInfo::detect() {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Error detecting system information: {}", e);
            std::process::exit(1);
        }
    };

    // Load configuration
    let (config, config_path) = match Config::load() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            eprintln!("\nSearched locations:");
            for location in Config::config_locations() {
                eprintln!("  - {}", location);
            }
            std::process::exit(1);
        }
    };

    if std::env::var("SOURCERY_DEBUG").is_ok() {
        eprintln!("Loaded config from: {}", config_path.display());
    }

    // Handle top-level flags
    if cli.update {
        commands::handle_update_repo(&config, &system_info);
        return;
    }

    if cli.clean {
        commands::handle_clean(&config, &system_info);
        return;
    }

    if cli.health {
        commands::handle_health(&config, &system_info);
        return;
    }

    // Handle subcommands
    match cli.command {
        Some(Commands::List { installed, upgradable }) => {
            commands::handle_list(&config, &system_info, installed, upgradable);
        }
        Some(Commands::Search { query, fuzzy, exact, package, collection }) => {
            commands::handle_search(&config, &system_info, &query, fuzzy, exact, package, collection);
        }
        Some(Commands::Build { package, branch, tag, container, local, chroot, confirm, verbose, noconfirm }) => {
            commands::handle_build(&config, &system_info, &package, branch, tag, container, local, chroot, confirm, verbose, noconfirm);
        }
        Some(Commands::Install { package, branch, tag, user, system, confirm, verbose, noconfirm }) => {
            commands::handle_install(&config, &system_info, &package, branch, tag, user, system, confirm, verbose, noconfirm);
        }
        Some(Commands::Update { package, branch, tag, confirm, verbose, noconfirm }) => {
            commands::handle_update(&config, &system_info, &package, branch, tag, confirm, verbose, noconfirm);
        }
        Some(Commands::Uninstall { package, confirm, verbose, noconfirm }) => {
            commands::handle_uninstall(&config, &system_info, &package, confirm, verbose, noconfirm);
        }
        Some(Commands::Purge { package, confirm, verbose, noconfirm }) => {
            commands::handle_purge(&config, &system_info, &package, confirm, verbose, noconfirm);
        }
        None => {
            eprintln!("No command specified. Use --help for usage information.");
            std::process::exit(1);
        }
    }
}
