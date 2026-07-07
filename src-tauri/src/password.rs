//! App-lock password hashing (Argon2id) and the Linux polkit-based admin reset.

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;

pub fn hash(plain: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify(plain: &str, encoded_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(encoded_hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok()
}

/// Authenticates the user as a system admin to clear a forgotten app-lock
/// password. Each platform prompts with the OS's own credential dialog and
/// returns whether the user successfully authenticated.
///
/// - Linux: polkit (`pkexec`) — prompts for the user/admin password.
/// - macOS: an AppleScript `do shell script … with administrator privileges`,
///   which raises the native admin-auth dialog.
/// - Windows: a UAC-elevated no-op via PowerShell `Start-Process -Verb RunAs`;
///   the elevation prompt is the admin gate, and the child's exit code tells us
///   whether the user consented rather than cancelled.
#[cfg(target_os = "linux")]
pub fn reset_with_admin() -> bool {
    std::process::Command::new("pkexec")
        .arg("/bin/true")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
pub fn reset_with_admin() -> bool {
    std::process::Command::new("osascript")
        .args(["-e", "do shell script \"true\" with administrator privileges"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub fn reset_with_admin() -> bool {
    // `Start-Process -Verb RunAs` triggers UAC; `-Wait -PassThru` lets us read
    // the elevated child's exit code. A cancelled UAC prompt makes Start-Process
    // throw, so the outer script exits non-zero and we report failure.
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "try { $p = Start-Process cmd.exe -ArgumentList '/c','exit 0' -Verb RunAs -Wait -PassThru; exit $p.ExitCode } catch { exit 1 }",
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
