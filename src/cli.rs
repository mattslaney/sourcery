use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "sourcery")]
#[command(version)]
#[command(about = "A package manager for maintaining system packages from source", long_about = None)]
pub struct Cli {
    /// Set log level (trace, debug, info, warning, error)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Update the package repositories
    #[arg(long)]
    pub update: bool,

    /// Clean out old sources and artifacts
    #[arg(long)]
    pub clean: bool,

    /// Check health of scry (dependencies, container runtime, etc)
    #[arg(long)]
    pub health: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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

        /// Show detailed information about the package/collection
        #[arg(long)]
        info: bool,
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

        /// Force rebuild even if artifacts already exist
        #[arg(long)]
        force: bool,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation (for unattended execution)
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

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation (for unattended execution)
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

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation (for unattended execution)
        #[arg(long)]
        noconfirm: bool,
    },

    /// Uninstall a package
    Uninstall {
        /// Package name to uninstall
        package: String,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation (for unattended execution)
        #[arg(long)]
        noconfirm: bool,
    },

    /// Uninstall and purge all package data
    Purge {
        /// Package name to purge
        package: String,

        /// Be verbose
        #[arg(long)]
        verbose: bool,

        /// Don't ask for any confirmation (for unattended execution)
        #[arg(long)]
        noconfirm: bool,
    },
}
