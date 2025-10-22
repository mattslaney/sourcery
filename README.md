# Sourcery
A package manager for maintaining system packages from source

# Usage
```shell
scry --update
scry --clean

# Search for a package/collection fuzzy or exact
scry search neovim
scry search --fuzzy neovim
scry search --exact neovim
scry search --exact neovim --package
scry search --fuzzy cli --collection
scry search --exact clidev --collection

# Build a package in a container, locally or in a chroot
scry build neovim 
scry build neovim --branch
scry build neovim --tag
scry build --container neovim
scry build --container "debian:latest" neovim
scry build --local neovim
scry build --chroot
scry build neovim --confirm --verbose
scry build neovim --noconfirm

# Install the package for the user or system
scry install neovim
scry install neovim --branch
scry install neovim --tag
scry install --user
scry install --system
scry install --system --confirm --verbose
scry install --system --noconfirm

# Update the package
scry update neovim
scry update neovim --branch
scry update neovim --tag
scry update neovim --confirm --verbose
scry update neovim --noconfirm

# Uninstall the package
scry uninstall neovim
scry uninstall neovim --confirm --verbose
scry uninstall neovim --noconfirm

## Uninstall and purge all package data
scry purge neovim
scry purge neovim --confirm --verbose
scry purge neovim --noconfirm
```

# Configuration
~/.config/sourcery/sourcery.toml
```toml
local_storage = '~/.local/share/sourcery'

[install]
user_path = '~/.local/bin'
system_path = '/usr/local/bin'

[build]
default_environment = "container"
default_container = "ubuntu:latest"

[repositories]
main = "http://github.com/mattslaney/sourcery-repository.git"
```

# Local Storage
The local storage directory will be used for storage of package repositories and the output of builds
 - ~/.local/share/sourcery/repositories/<repositoryname>
   directory containing sourcery package repositories
 - ~/.local/share/sourcery/source/<package>
   directory containing package source code for local builds
 - ~/.local/share/sourcery/outputs/<package>
   directory containing resulting build artifacts
 - ~/.local/share/sourcery/logs/sourcery.log
   the log file for the sourcery application
 - ~/.local/share/sourcery/logs/audit.log
   an audit log of sourcery commands and results
 - ~/.local/share/sourcery/logs/packages/<package>/build.log
   the build logs for a package

