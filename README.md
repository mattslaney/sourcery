# Sourcery
A package manager for maintaining system packages from source.

# Usage
```shell
scry --update # Update the package repositories
scry --clean # Clean out old sources and outputs
scry --health # check health of scry (dependencies, container runtime, etc)
scry list --installed # List packages installed with scry
scry list --upgradable # List installed packages that are upgradable

# Search for a package/collection fuzzy or exact
scry search neovim # default search - fuzzy
scry search --fuzzy neovim
scry search --exact neovim # search for exact package name
scry search --exact neovim --package 
scry search --fuzzy cli --collection # search for a collection
scry search --exact clidev --collection  # search for an exact collection

# Build a package in a container, locally or in a chroot
scry build neovim # build a package
scry build neovim --branch BRANCH # build a package from a branch
scry build neovim --tag TAG # build a package from a tag
scry build --container neovim # build in a container (default same os as current system)
scry build --container "debian:latest" neovim # build using a specific container
scry build --local neovim # build on current system
scry build --chroot # build in a chroot
scry build neovim --confirm --verbose # confirm before commands and be verbose
scry build neovim --noconfirm # don't ask for any confirmnation

# Install the package for the user or system
scry install neovim # install the package
scry install neovim --branch # install the package from a branch
scry install neovim --tag # install a package from a tag
scry install --user # install for the current user
scry install --system # install systemn-wide
scry install --system --confirm --verbose # install with confirmation and be verbose
scry install --system --noconfirm # don't ask for any confirmation

# Update the package
scry update neovim # update a package
scry update neovim --branch # update a package with build from a branch
scry update neovim --tag # update a package  with build from a branch
scry update neovim --confirm --verbose # update with confirmation steps anbd be verbose
scry update neovim --noconfirm # update without any confirmation

# Uninstall the package
scry uninstall neovim # uninstall a package
scry uninstall neovim --confirm --verbose # uninstall with confirmation and be verbose
scry uninstall neovim --noconfirm # uninstall without any.confirmation

## Uninstall and purge all package data
scry purge neovim # purge a package
scry purge neovim --confirm --verbose # purge with confirmation and be verbose
scry purge neovim --noconfirm # purge without any.confirmation
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

# Sourcery install location
## system-wide
 - /usr/local/bin/sourcery # the sourcery binary
 - /usr/local/bin/scry -> /usr/local/bin/sourcery # short alias symlink to sourcery
 - /etc/sourcery/containers # containerfiles used by sourcery for build