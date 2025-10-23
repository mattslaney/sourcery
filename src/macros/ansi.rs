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

