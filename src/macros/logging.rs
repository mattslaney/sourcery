#[macro_export]
macro_rules! style {
    ($style:expr, $($arg:tt)*) => {
        {
            let styles: Vec<&str> = $style.split(',').collect();
            let mut combined_styles = String::new();

            for s in styles {
                match s.trim() {
                    "black" => combined_styles.push_str("\x1b[30m"),
                    "red" => combined_styles.push_str("\x1b[31m"),
                    "green" => combined_styles.push_str("\x1b[32m"),
                    "yellow" => combined_styles.push_str("\x1b[33m"),
                    "orange" => combined_styles.push_str("\x1b[38;5;208m"), // 256-color orange
                    "blue" => combined_styles.push_str("\x1b[34m"),
                    "magenta" => combined_styles.push_str("\x1b[35m"),
                    "cyan" => combined_styles.push_str("\x1b[36m"),
                    "white" => combined_styles.push_str("\x1b[37m"),
                    "default" => combined_styles.push_str("\x1b[39m"),
                    "bold" => combined_styles.push_str("\x1b[1m"),
                    "dim" => combined_styles.push_str("\x1b[2m"),
                    "italic" => combined_styles.push_str("\x1b[3m"),
                    "underline" => combined_styles.push_str("\x1b[4m"),
                    "blink" => combined_styles.push_str("\x1b[5m"),
                    "reverse" => combined_styles.push_str("\x1b[7m"),
                    "hide" => combined_styles.push_str("\x1b[8m"),
                    _ => (),
                }
            }

            format!("{}{}\x1b[0m", combined_styles, format_args!($($arg)*))
        }
    };
}

/** Colour macros */

#[macro_export]
macro_rules! black {
    ($($arg:tt)*) => {
        style!("black", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! red {
    ($($arg:tt)*) => {
        style!("red", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! green {
    ($($arg:tt)*) => {
        style!("green", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! yellow {
    ($($arg:tt)*) => {
        style!("yellow", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! orange {
    ($($arg:tt)*) => {
        style!("orange", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! blue {
    ($($arg:tt)*) => {
        style!("blue", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! magenta {
    ($($arg:tt)*) => {
        style!("magenta", "{}", format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! cyan {
    ($($arg:tt)*) => {
        style!("cyan", "{}", format_args!($($arg)*))
    };
}

/** Logging macros */
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            let log = std::env::var("RUST_LOG").unwrap_or(String::from("info"));
            println!("log = {}", log);
            if log.contains("trace") {
                println!("{}{}", style!("dim,magenta", "[TRACE] "), style!("dim,white", "{}", format_args!($($arg)*)));
            }
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            let log = std::env::var("RUST_LOG").unwrap_or(String::from("info"));
            if log.contains("debug") || log.contains("trace") {
                println!("{}{}", style!("magenta", "[DEBUG] "), style!("dim,white", "{}", format_args!($($arg)*)));
            }
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            let log = std::env::var("RUST_LOG").unwrap_or(String::from("info"));
            if log.contains("info") || log.contains("debug") || log.contains("trace") {
                println!("{}{}", style!("cyan", "[INFO] "), format_args!($($arg)*));
            }
        }
    };
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            let log = std::env::var("RUST_LOG").unwrap_or(String::from("info"));
            if log.contains("warning") || log.contains("info") || log.contains("debug") || log.contains("trace") {
                println!("{}{}", style!("yellow", "[WARN] "), format_args!($($arg)*));
            }
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            let log = std::env::var("RUST_LOG").unwrap_or(String::from("info"));
            if log.contains("error") || log.contains("warning") || log.contains("info") || log.contains("debug") || log.contains("trace") {
                println!("{}{}", style!("red", "[ERROR] "), format_args!($($arg)*));
            }
        }
    };
}

#[macro_export]
macro_rules! failure {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            println!("{}{}", style!("red", "[FAILURE] "), format_args!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            println!("{}{}", style!("green", "[SUCCESS] "), format_args!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! note {
    ($($arg:tt)*) => {
        if cfg!(feature = "log") {
            let log = std::env::var("RUST_LOG").unwrap_or(String::from("info"));
            if log.contains("note") {
                println!("{}{}", style!("dim,blue", "[NOTE] "), style!("dim,white", "{}", format_args!($($arg)*)));
            }
        }
    };
}

#[macro_export]
macro_rules! msg {
    () => {
        println!();
    };
    ($($arg:tt)*) => {
        println!("{}", format_args!($($arg)*));
    };
}
