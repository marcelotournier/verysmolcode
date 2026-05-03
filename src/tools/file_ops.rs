//! File-system safety helpers shared across the new pi-style tools.
//!
//! The actual `read`/`write`/`edit`/`ls` tool implementations live in their
//! own modules (`crate::tools::read`, `::write`, `::edit`, `::ls`) — pi
//! organizes tools the same way. This module retains only the path-safety
//! constants and the helper used by `write` and `edit` to refuse mutations
//! against system directories or sensitive home-dir dotfiles.

use std::path::Path;

/// System paths that should never be written to by the coding assistant.
/// Single source of truth — also referenced by `is_dangerous_tool_call()`.
pub const BLOCKED_PATH_PREFIXES: &[&str] = &[
    "/etc/", "/boot/", "/usr/", "/bin/", "/sbin/", "/lib/", "/proc/", "/sys/", "/dev/",
];

pub fn check_safe_path(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    for b in BLOCKED_PATH_PREFIXES {
        if path_str.starts_with(b) {
            return Err(format!("Access denied: {} is a protected path", b));
        }
    }
    if let Some(home) = dirs::home_dir() {
        let dangerous_dotfiles = [".bashrc", ".profile", ".bash_profile", ".ssh", ".gnupg"];
        for df in &dangerous_dotfiles {
            if path == home.join(df) {
                return Err(format!("Access denied: modifying ~/{} is not allowed", df));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_path_normal() {
        assert!(check_safe_path(Path::new("/tmp/test.txt")).is_ok());
        assert!(check_safe_path(Path::new("/home/user/project/src/main.rs")).is_ok());
    }

    #[test]
    fn test_safe_path_blocks_etc() {
        assert!(check_safe_path(Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_usr() {
        assert!(check_safe_path(Path::new("/usr/local/bin/app")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_proc() {
        assert!(check_safe_path(Path::new("/proc/1/status")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_sys() {
        assert!(check_safe_path(Path::new("/sys/class/net")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_home_dotfiles() {
        if let Some(home) = dirs::home_dir() {
            assert!(check_safe_path(&home.join(".bashrc")).is_err());
            assert!(check_safe_path(&home.join(".ssh")).is_err());
            assert!(check_safe_path(&home.join(".gnupg")).is_err());
        }
    }
}
