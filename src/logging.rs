use std::sync::OnceLock;

/// Global log level storage
static LOG_LEVEL: OnceLock<String> = OnceLock::new();

/// Initialize the global log level
pub fn init_log_level(level: &str) {
    let _ = LOG_LEVEL.set(level.to_lowercase());
}

/// Get the current log level
pub fn get_log_level() -> &'static str {
    LOG_LEVEL.get().map(|s| s.as_str()).unwrap_or("info")
}

/// Check if a log level should be displayed based on current level
pub fn should_log(level: &str) -> bool {
    let current = get_log_level();
    
    // Define log level hierarchy
    let levels = ["error", "warning", "info", "debug", "trace"];
    
    let current_idx = levels.iter().position(|&l| l == current);
    let level_idx = levels.iter().position(|&l| l == level);
    
    match (current_idx, level_idx) {
        (Some(cur), Some(lvl)) => lvl <= cur,
        _ => false,
    }
}

/// Log a trace message (most verbose)
pub fn trace(message: impl std::fmt::Display) {
    if should_log("trace") {
        println!("{}{}", style!("dim,magenta", "[TRACE] "), style!("dim,white", "{}", message));
    }
}

/// Log a debug message
pub fn debug(message: impl std::fmt::Display) {
    if should_log("debug") {
        println!("{}{}", style!("magenta", "[DEBUG] "), style!("dim,white", "{}", message));
    }
}

/// Log an info message
pub fn info(message: impl std::fmt::Display) {
    if should_log("info") {
        println!("{}{}", style!("cyan", "[INFO] "), message);
    }
}

/// Log a warning message
pub fn warning(message: impl std::fmt::Display) {
    if should_log("warning") {
        println!("{}{}", style!("yellow", "[WARN] "), message);
    }
}

/// Log an error message
pub fn error(message: impl std::fmt::Display) {
    if should_log("error") {
        println!("{}{}", style!("red", "[ERROR] "), message);
    }
}

/// Log a note message (shown at debug level or higher)
pub fn note(message: impl std::fmt::Display) {
    if should_log("debug") {
        println!("{}{}", style!("dim,blue", "[NOTE] "), style!("dim,white", "{}", message));
    }
}

