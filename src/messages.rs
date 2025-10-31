/// Print a message to the user
pub fn msg(message: impl std::fmt::Display) {
    println!("{}", message);
}

/// Print additional information to the user
pub fn info(message: impl std::fmt::Display) {
    println!("{}", style!("dim, white", "{}", message.to_string()));
}

/// Print a blank line
pub fn blank() {
    println!();
}

/// Display a success message to the user
pub fn success(message: impl std::fmt::Display) {
    println!("{}", style!("green", "{}", message.to_string()));
}

/// Display a failure message to the user
pub fn failure(message: impl std::fmt::Display) {
    println!("{}", style!("red", "{}", message.to_string()));
}

/// Display a caution message to the user
pub fn caution(message: impl std::fmt::Display) {
    println!("{}", style!("yellow", "{}", message.to_string()));
}

/// Ask the user for confirmation (returns true if user confirms)
pub fn confirm(message: impl std::fmt::Display) -> bool {
    use std::io::{self, Write};
    
    print!("{} [y/N]: ", style!("yellow", "{}", message.to_string()));
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

