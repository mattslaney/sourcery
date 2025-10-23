use crate::config::Config;
use crate::system::SystemInfo;

pub fn handle_list(_config: &Config, _system_info: &SystemInfo, installed: bool, upgradable: bool) {
    if installed {
        println!("Listing installed packages...");
        // TODO: Implement list installed logic
    } else if upgradable {
        println!("Listing upgradable packages...");
        // TODO: Implement list upgradable logic
    } else {
        eprintln!("Error: Please specify --installed or --upgradable");
        std::process::exit(1);
    }
}

