use crate::config::Config;
use crate::system::SystemInfo;

#[allow(clippy::too_many_arguments)]
pub fn handle_build(
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
    println!(
        "  Target system: {} {} ({})",
        system_info.distro_name, system_info.distro_version_id, system_info.arch
    );

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
    println!(
        "  Confirm: {}, Verbose: {}, NoConfirm: {}",
        confirm, verbose, noconfirm
    );

    // TODO: Implement build logic
}
