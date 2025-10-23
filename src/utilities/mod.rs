pub mod build_env;
pub mod collection;
pub mod container;
pub mod git;
pub mod package;

#[cfg(test)]
mod integration_test;

pub use build_env::{BuildEnvironment, format_env_for_container, get_build_volume_mounts};
pub use collection::{Collection, load_collection};
pub use container::{ContainerRuntime, ContainerRuntimeType, SourceryImageBuilder};
pub use git::{GitRepo, is_git_available, get_git_version};
pub use package::{Package, PackageStage, load_package};
