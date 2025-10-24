pub mod build_env;
pub mod collection;
pub mod container;
pub mod git;
pub mod package;
pub mod stage;

#[cfg(test)]
mod integration_test;

pub use build_env::{BuildEnvironment, format_env_for_container, get_build_volume_mounts};
pub use collection::load_collection;
pub use container::{ContainerRuntime, ContainerRuntimeType, SourceryImageBuilder};
pub use git::GitRepo;
pub use package::{Package, PackageStage, load_package};
pub use stage::resolve_prerequisites;
