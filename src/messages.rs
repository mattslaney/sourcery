/// Print a message to the user
pub fn msg(message: impl std::fmt::Display) {
    println!("{}", message);
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

