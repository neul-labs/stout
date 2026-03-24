//! Output formatting utilities
#![allow(dead_code)]

use console::{style, Style};

/// Style for success messages
pub fn success() -> Style {
    Style::new().green()
}

/// Style for error messages
pub fn error() -> Style {
    Style::new().red().bold()
}

/// Style for warnings
pub fn warning() -> Style {
    Style::new().yellow()
}

/// Style for info/actions
pub fn info() -> Style {
    Style::new().cyan()
}

/// Style for muted/secondary text
pub fn muted() -> Style {
    Style::new().dim()
}

/// Format a package name
pub fn package(name: &str) -> String {
    style(name).green().to_string()
}

/// Format a version
pub fn version(ver: &str) -> String {
    style(ver).dim().to_string()
}

/// Print an error message and exit
pub fn fatal(msg: &str) -> ! {
    eprintln!("\n{} {}\n", style("error:").red().bold(), msg);
    std::process::exit(1)
}

/// Check if stdin is a TTY (interactive terminal).
pub fn is_interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}
