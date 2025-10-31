use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Represents the available container runtimes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerRuntimeType {
    Podman,
    Docker,
}

impl ContainerRuntimeType {
    /// Get the command name for this runtime
    pub fn command(&self) -> &str {
        match self {
            ContainerRuntimeType::Podman => "podman",
            ContainerRuntimeType::Docker => "docker",
        }
    }

    /// Get a human-readable name for this runtime
    pub fn name(&self) -> &str {
        match self {
            ContainerRuntimeType::Podman => "Podman",
            ContainerRuntimeType::Docker => "Docker",
        }
    }
}

/// Container runtime abstraction for building and running containers
#[derive(Clone)]
pub struct ContainerRuntime {
    runtime_type: ContainerRuntimeType,
}

impl ContainerRuntime {
    /// Create a new ContainerRuntime with the specified type
    pub fn new(runtime_type: ContainerRuntimeType) -> Self {
        Self { runtime_type }
    }

    /// Detect and return the first available container runtime
    /// Priority: Podman > Docker
    pub fn detect() -> Result<Self, String> {
        if Self::is_available(ContainerRuntimeType::Podman) {
            Ok(Self::new(ContainerRuntimeType::Podman))
        } else if Self::is_available(ContainerRuntimeType::Docker) {
            Ok(Self::new(ContainerRuntimeType::Docker))
        } else {
            Err("No container runtime found. Please install podman or docker.".to_string())
        }
    }

    /// Check if a specific runtime is available on the system
    pub fn is_available(runtime: ContainerRuntimeType) -> bool {
        Command::new(runtime.command())
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Get the runtime type
    pub fn runtime_type(&self) -> ContainerRuntimeType {
        self.runtime_type
    }

    /// Get the command name for this runtime
    pub fn command(&self) -> &str {
        self.runtime_type.command()
    }

    /// Build a container image from a Containerfile
    ///
    /// # Arguments
    /// * `containerfile` - Path to the Containerfile
    /// * `tag` - Tag for the built image (e.g., "sourcery-build-ubuntu")
    /// * `build_args` - Optional build arguments (e.g., VERSION=22.04)
    pub fn build_image(
        &self,
        containerfile: &Path,
        tag: &str,
        build_args: Option<&[(&str, &str)]>,
    ) -> Result<Output, std::io::Error> {
        let mut cmd = Command::new(self.command());
        cmd.arg("build")
            .arg("-f")
            .arg(containerfile)
            .arg("-t")
            .arg(tag);

        // Add build arguments if provided
        if let Some(args) = build_args {
            for (key, value) in args {
                cmd.arg("--build-arg").arg(format!("{}={}", key, value));
            }
        }

        // Add context (directory containing Containerfile)
        if let Some(context_dir) = containerfile.parent() {
            cmd.arg(context_dir);
        } else {
            cmd.arg(".");
        }

        cmd.output()
    }

    /// Check if an image exists locally
    pub fn image_exists(&self, tag: &str) -> bool {
        Command::new(self.command())
            .arg("image")
            .arg("inspect")
            .arg(tag)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// List all images with a specific prefix
    pub fn list_images(&self, prefix: Option<&str>) -> Result<Vec<String>, std::io::Error> {
        let output = Command::new(self.command())
            .arg("images")
            .arg("--format")
            .arg("{{.Repository}}:{{.Tag}}")
            .output()?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let images: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                if let Some(prefix) = prefix {
                    if line.starts_with(prefix) {
                        Some(line.to_string())
                    } else {
                        None
                    }
                } else {
                    Some(line.to_string())
                }
            })
            .collect();

        Ok(images)
    }

    /// Run a command inside a container with volume mounts
    ///
    /// # Arguments
    /// * `image` - The container image to use
    /// * `command` - The command to run inside the container
    /// * `volumes` - List of (host_path, container_path) tuples for volume mounts
    /// * `env_vars` - Optional environment variables to set
    pub fn run(
        &self,
        image: &str,
        command: &str,
        volumes: &[(PathBuf, PathBuf)],
        env_vars: Option<&[(&str, &str)]>,
    ) -> Result<Output, std::io::Error> {
        let mut cmd = Command::new(self.command());
        cmd.arg("run").arg("--rm"); // Remove container after execution
        
        // Keep user namespace mapping (important for file permissions)
        // This is only supported by Podman
        if self.runtime_type == ContainerRuntimeType::Podman {
            cmd.arg("--userns=keep-id");
        }

        // Add volume mounts
        for (host_path, container_path) in volumes {
            cmd.arg("-v").arg(format!(
                "{}:{}",
                host_path.display(),
                container_path.display()
            ));
        }

        // Add environment variables
        if let Some(vars) = env_vars {
            for (key, value) in vars {
                cmd.arg("-e").arg(format!("{}={}", key, value));
            }
        }

        // Add image and command
        cmd.arg(image).arg(command);

        cmd.output()
    }

    /// Remove a container image
    pub fn remove_image(&self, tag: &str) -> Result<Output, std::io::Error> {
        Command::new(self.command())
            .arg("rmi")
            .arg(tag)
            .output()
    }

    /// Get the version of the container runtime
    pub fn version(&self) -> Result<String, std::io::Error> {
        let output = Command::new(self.command()).arg("--version").output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get version",
            ))
        }
    }
}

/// Helper struct for building Sourcery base images
pub struct SourceryImageBuilder {
    runtime: ContainerRuntime,
    containerfiles_dir: PathBuf,
}

impl SourceryImageBuilder {
    /// Create a new SourceryImageBuilder
    ///
    /// # Arguments
    /// * `runtime` - The container runtime to use
    /// * `containerfiles_dir` - Directory containing Containerfiles (default: /etc/sourcery/containers)
    pub fn new(runtime: ContainerRuntime, containerfiles_dir: PathBuf) -> Self {
        Self {
            runtime,
            containerfiles_dir,
        }
    }

    /// Build a base image for a specific distribution
    ///
    /// # Arguments
    /// * `distro_id` - Distribution ID (e.g., "ubuntu", "fedora")
    /// * `version` - Optional version (e.g., "22.04", "38"). Uses "latest" if None
    pub fn build_base_image(
        &self,
        distro_id: &str,
        version: Option<&str>,
    ) -> Result<String, String> {
        let containerfile = self
            .containerfiles_dir
            .join(format!("{}.Containerfile", distro_id));

        if !containerfile.exists() {
            return Err(format!(
                "Containerfile not found for distribution: {}",
                distro_id
            ));
        }

        let version = version.unwrap_or("latest");
        let tag = format!("sourcery-build-{}:{}", distro_id, version);

        // Build the image (container runtime will use layer caching if nothing changed)
        let build_args = [("VERSION", version)];
        let output = self
            .runtime
            .build_image(&containerfile, &tag, Some(&build_args))
            .map_err(|e| format!("Failed to build image: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to build image: {}", stderr));
        }

        Ok(tag)
    }

    /// Get the tag for a base image (doesn't build it)
    pub fn get_base_image_tag(distro_id: &str, version: Option<&str>) -> String {
        let version = version.unwrap_or("latest");
        format!("sourcery-build-{}:{}", distro_id, version)
    }

    /// Check if a base image exists
    pub fn base_image_exists(&self, distro_id: &str, version: Option<&str>) -> bool {
        let tag = Self::get_base_image_tag(distro_id, version);
        self.runtime.image_exists(&tag)
    }

    /// List all Sourcery base images
    pub fn list_base_images(&self) -> Result<Vec<String>, std::io::Error> {
        self.runtime.list_images(Some("sourcery-build-"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_type_command() {
        assert_eq!(ContainerRuntimeType::Podman.command(), "podman");
        assert_eq!(ContainerRuntimeType::Docker.command(), "docker");
    }

    #[test]
    fn test_runtime_type_name() {
        assert_eq!(ContainerRuntimeType::Podman.name(), "Podman");
        assert_eq!(ContainerRuntimeType::Docker.name(), "Docker");
    }

    #[test]
    fn test_container_runtime_creation() {
        let runtime = ContainerRuntime::new(ContainerRuntimeType::Podman);
        assert_eq!(runtime.runtime_type(), ContainerRuntimeType::Podman);
        assert_eq!(runtime.command(), "podman");
    }

    #[test]
    fn test_get_base_image_tag() {
        assert_eq!(
            SourceryImageBuilder::get_base_image_tag("ubuntu", Some("22.04")),
            "sourcery-build-ubuntu:22.04"
        );
        assert_eq!(
            SourceryImageBuilder::get_base_image_tag("fedora", None),
            "sourcery-build-fedora:latest"
        );
    }

    // Integration tests that require actual container runtime
    #[test]
    #[ignore] // Run with `cargo test -- --ignored` if you have podman/docker installed
    fn test_detect_runtime() {
        let result = ContainerRuntime::detect();
        // This will pass if either podman or docker is installed
        if result.is_ok() {
            let runtime = result.unwrap();
            println!("Detected runtime: {:?}", runtime.runtime_type());
        }
    }

    #[test]
    #[ignore]
    fn test_runtime_version() {
        if let Ok(runtime) = ContainerRuntime::detect() {
            let version = runtime.version();
            assert!(version.is_ok());
            println!("Runtime version: {}", version.unwrap());
        }
    }
}

