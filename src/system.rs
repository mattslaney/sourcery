use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// System architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    /// Distribution ID (e.g., "ubuntu", "fedora", "debian")
    pub distro_id: String,
    /// Distribution name (e.g., "Ubuntu", "Fedora")
    pub distro_name: String,
    /// Distribution version (e.g., "22.04", "38")
    pub distro_version: String,
    /// Distribution version ID (e.g., "22.04", "38")
    pub distro_version_id: String,
    /// Distribution version codename (e.g., "jammy", "bookworm")
    pub distro_version_codename: Option<String>,
}

impl SystemInfo {
    /// Detect the current system information
    pub fn detect() -> Result<Self, Box<dyn std::error::Error>> {
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

