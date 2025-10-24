/// Security module for analyzing and validating commands from package definitions
/// 
/// This module implements a tiered security system:
/// - BLOCKED: Commands that will never be executed (no override)
/// - CAUTION: Commands that require extra confirmation and warnings

use regex::Regex;

/// Security level for a command
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    /// Command is safe to execute with normal confirmation
    Safe,
    /// Command requires extra warning and confirmation
    Caution,
    /// Command is blocked and will never be executed
    Blocked,
}

/// Result of security analysis
#[derive(Debug, Clone)]
pub struct SecurityAnalysis {
    pub level: SecurityLevel,
    pub reasons: Vec<String>,
}

impl SecurityAnalysis {
    pub fn new(level: SecurityLevel) -> Self {
        Self {
            level,
            reasons: Vec::new(),
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reasons.push(reason);
        self
    }
}

/// Analyze a command for security concerns
pub fn analyze_command(command: &str) -> SecurityAnalysis {
    let mut analysis = SecurityAnalysis::new(SecurityLevel::Safe);

    // Check for blocked patterns first
    if let Some(reason) = check_blocked_patterns(command) {
        analysis.level = SecurityLevel::Blocked;
        analysis.reasons.push(reason);
        return analysis;
    }

    // Check for caution patterns
    let caution_reasons = check_caution_patterns(command);
    if !caution_reasons.is_empty() {
        analysis.level = SecurityLevel::Caution;
        analysis.reasons = caution_reasons;
    }

    analysis
}

/// Check for blocked patterns that should never be executed
fn check_blocked_patterns(command: &str) -> Option<String> {
    // Normalize command for checking (replace multiple spaces, tabs with single space)
    let normalized = command.split_whitespace().collect::<Vec<_>>().join(" ");

    // Check for root filesystem deletion
    if contains_root_deletion(&normalized) {
        return Some("Attempting to remove root filesystem (/)".to_string());
    }

    // Check for home directory deletion
    if contains_home_deletion(&normalized) {
        return Some("Attempting to remove entire home directory (~)".to_string());
    }

    // Check for system directory operations
    if let Some(dir) = contains_system_directory_operation(&normalized) {
        return Some(format!("Attempting to modify critical system directory ({})", dir));
    }

    // Check for fork bombs
    if contains_fork_bomb(&normalized) {
        return Some("Fork bomb detected".to_string());
    }

    None
}

/// Check for patterns that require caution
fn check_caution_patterns(command: &str) -> Vec<String> {
    let mut reasons = Vec::new();

    // Check for recursive removal
    if contains_recursive_removal(command) {
        reasons.push("Recursive file deletion (rm -rf)".to_string());
    }

    // Check for dd command
    if contains_dd_command(command) {
        reasons.push("Direct disk operation (dd)".to_string());
    }

    // Check for download-and-execute
    if contains_download_execute(command) {
        reasons.push("Download and execute pattern (curl/wget | sh/bash)".to_string());
    }

    // Check for network data exfiltration
    if contains_network_post(command) {
        reasons.push("Network data transmission (POST/upload)".to_string());
    }

    // Check for privilege escalation
    if contains_privilege_escalation(command) {
        reasons.push("Privilege escalation attempt (sudo/su)".to_string());
    }

    // Check for wildcards in potentially dangerous commands
    if contains_dangerous_wildcards(command) {
        reasons.push("Wildcards in deletion commands".to_string());
    }

    reasons
}

/// Check if command attempts to delete root filesystem
fn contains_root_deletion(command: &str) -> bool {
    let patterns = [
        r"rm\s+.*-[rf]+.*\s+/+\s*$",           // rm -rf /
        r"rm\s+.*-[rf]+.*\s+/+\*",             // rm -rf /*
        r"rm\s+.*-[rf]+.*\s+/\*/",             // rm -rf /*/
        r"rm\s+.*-[rf]+.*\s+//+",              // rm -rf //
    ];

    patterns.iter().any(|p| {
        Regex::new(p).unwrap().is_match(command)
    })
}

/// Check if command attempts to delete home directory
fn contains_home_deletion(command: &str) -> bool {
    let patterns = [
        r"rm\s+.*-[rf]+.*\s+~+\s*$",           // rm -rf ~
        r"rm\s+.*-[rf]+.*\s+~/+\s*$",          // rm -rf ~/
        r"rm\s+.*-[rf]+.*\s+~/+\*",            // rm -rf ~/*
        r"rm\s+.*-[rf]+.*\s+\$HOME\s*$",       // rm -rf $HOME
        r"rm\s+.*-[rf]+.*\s+\$HOME/+\s*$",     // rm -rf $HOME/
        r"rm\s+.*-[rf]+.*\s+\$HOME/+\*",       // rm -rf $HOME/*
        r"rm\s+.*-[rf]+.*\s+\$\{HOME\}\s*$",   // rm -rf ${HOME}
        r"rm\s+.*-[rf]+.*\s+\$\{HOME\}/",      // rm -rf ${HOME}/
    ];

    patterns.iter().any(|p| {
        Regex::new(p).unwrap().is_match(command)
    })
}

/// Check if command operates on system directories
fn contains_system_directory_operation(command: &str) -> Option<&'static str> {
    let system_dirs = [
        "/etc", "/usr", "/var", "/sys", "/proc", 
        "/boot", "/bin", "/sbin", "/lib", "/lib64", 
        "/opt", "/root"
    ];

    for dir in &system_dirs {
        // Check for operations on these directories
        // Look for: rm, chmod, chown, mv, cp targeting these dirs
        let patterns = [
            format!(r"rm\s+.*\s+{}", regex::escape(dir)),
            format!(r"rm\s+.*\s+{}/", regex::escape(dir)),
            format!(r"chmod\s+.*\s+{}", regex::escape(dir)),
            format!(r"chown\s+.*\s+{}", regex::escape(dir)),
        ];

        for pattern in &patterns {
            if Regex::new(pattern).unwrap().is_match(command) {
                return Some(dir);
            }
        }
    }

    None
}

/// Check for fork bomb patterns
fn contains_fork_bomb(command: &str) -> bool {
    // Classic fork bomb: :(){ :|:& };:
    command.contains(":|:&") || command.contains(":(){")
}

/// Check for recursive removal commands
fn contains_recursive_removal(command: &str) -> bool {
    // Only flag if it contains -r (with or without f), not just -f alone
    // Use word boundary to avoid matching 'r' in filenames
    let patterns = [
        r"rm\s+.*-[a-z]*r[a-z]*\s",  // -r, -rf, -fr, etc. followed by space
        r"rm\s+.*-[a-z]*r[a-z]*$",   // -r, -rf, -fr, etc. at end of command
        r"rm\s+.*--recursive",       // --recursive
    ];
    
    patterns.iter().any(|p| {
        Regex::new(p).unwrap().is_match(command)
    })
}

/// Check for dd commands
fn contains_dd_command(command: &str) -> bool {
    Regex::new(r"\bdd\s+").unwrap().is_match(command)
}

/// Check for download-and-execute patterns
fn contains_download_execute(command: &str) -> bool {
    let patterns = [
        r"curl\s+.*\|\s*(sh|bash|zsh|fish)",
        r"wget\s+.*\|\s*(sh|bash|zsh|fish)",
        r"curl\s+.*-o\s*-.*\|\s*(sh|bash)",
    ];

    patterns.iter().any(|p| {
        Regex::new(p).unwrap().is_match(command)
    })
}

/// Check for network POST/upload operations
fn contains_network_post(command: &str) -> bool {
    let patterns = [
        r"curl\s+.*-X\s+POST",
        r"curl\s+.*--data",
        r"curl\s+.*-d\s+",
        r"wget\s+.*--post-data",
        r"wget\s+.*--post-file",
    ];

    patterns.iter().any(|p| {
        Regex::new(p).unwrap().is_match(command)
    })
}

/// Check for privilege escalation attempts
fn contains_privilege_escalation(command: &str) -> bool {
    let patterns = [
        r"\bsudo\s+",
        r"\bsu\s+",
        r"chmod\s+.*[+]s",  // SUID bit
    ];

    patterns.iter().any(|p| {
        Regex::new(p).unwrap().is_match(command)
    })
}

/// Check for dangerous wildcards
fn contains_dangerous_wildcards(command: &str) -> bool {
    // Look for rm with wildcards (any * in the command with rm)
    if command.contains("rm") && command.contains('*') {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_root_deletion() {
        let commands = [
            "rm -rf /",
            "rm -rf /*",
            "rm -rf /*/",
            "rm -rf //",
            "rm -f -r /",
        ];

        for cmd in &commands {
            let analysis = analyze_command(cmd);
            assert_eq!(analysis.level, SecurityLevel::Blocked, "Failed for: {}", cmd);
        }
    }

    #[test]
    fn test_blocked_home_deletion() {
        let commands = [
            "rm -rf ~",
            "rm -rf ~/",
            "rm -rf ~/*",
            "rm -rf $HOME",
            "rm -rf $HOME/",
            "rm -rf $HOME/*",
            "rm -rf ${HOME}",
        ];

        for cmd in &commands {
            let analysis = analyze_command(cmd);
            assert_eq!(analysis.level, SecurityLevel::Blocked, "Failed for: {}", cmd);
        }
    }

    #[test]
    fn test_blocked_system_directories() {
        let commands = [
            "rm -rf /etc",
            "rm -rf /usr",
            "rm -rf /var/",
            "chmod -R 777 /etc",
        ];

        for cmd in &commands {
            let analysis = analyze_command(cmd);
            assert_eq!(analysis.level, SecurityLevel::Blocked, "Failed for: {}", cmd);
        }
    }

    #[test]
    fn test_caution_patterns() {
        let commands = [
            ("rm -rf ~/.cache/*", true),
            ("rm -rf ./build", true),
            ("dd if=/dev/zero of=file", true),
            ("curl http://example.com | sh", true),
            ("curl -X POST -d 'data' http://example.com", true),
            ("sudo apt install", true),
        ];

        for (cmd, should_caution) in &commands {
            let analysis = analyze_command(cmd);
            if *should_caution {
                assert_eq!(analysis.level, SecurityLevel::Caution, "Failed for: {}", cmd);
                assert!(!analysis.reasons.is_empty(), "Should have reasons for: {}", cmd);
            }
        }
    }

    #[test]
    fn test_safe_commands() {
        let commands = [
            "make",
            "./configure --prefix=/usr/local",
            "mkdir -p ./build",
            "cp file1 file2",
            "chmod +x ./script.sh",
        ];

        for cmd in &commands {
            let analysis = analyze_command(cmd);
            // These should be safe (no wildcards in rm, no dangerous patterns)
            assert_eq!(analysis.level, SecurityLevel::Safe, "Failed for: {}", cmd);
        }
    }

    #[test]
    fn test_specific_file_removal() {
        // Specific file removals
        // rm -f (non-recursive) should be safe
        let safe_commands = [
            "rm -f ~/.tmux.conf",
            "rm -f ~/.vimrc",
        ];

        for cmd in &safe_commands {
            let analysis = analyze_command(cmd);
            assert_eq!(analysis.level, SecurityLevel::Safe, "Failed for: {}", cmd);
        }

        // rm -rf even on specific directories should trigger caution
        let caution_commands = [
            "rm -rf ~/.config/tmux",
        ];

        for cmd in &caution_commands {
            let analysis = analyze_command(cmd);
            assert_eq!(analysis.level, SecurityLevel::Caution, "Failed for: {}", cmd);
        }
    }
}

