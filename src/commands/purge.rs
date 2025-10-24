use crate::config::Config;
use crate::system::SystemInfo;

pub fn handle_purge(
    _config: &Config,
    _system_info: &SystemInfo,
    package: &str,
    verbose: bool,
    noconfirm: bool,
) {
    println!("Purging package: {}", package);
    println!(
        "  Verbose: {}, NoConfirm: {}",
        verbose, noconfirm
    );

    // TODO: Implement purge logic
}
