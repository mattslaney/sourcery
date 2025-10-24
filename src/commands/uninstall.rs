use crate::config::Config;
use crate::system::SystemInfo;

pub fn handle_uninstall(
    _config: &Config,
    _system_info: &SystemInfo,
    package: &str,
    verbose: bool,
    noconfirm: bool,
) {
    println!("Uninstalling package: {}", package);
    println!(
        "  Verbose: {}, NoConfirm: {}",
        verbose, noconfirm
    );

    // TODO: Implement uninstall logic
}
