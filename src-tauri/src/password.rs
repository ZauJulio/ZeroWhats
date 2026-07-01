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
/// password. Implemented with polkit (pkexec) on Linux; on Windows/macOS there
/// is no equivalent one-liner, so the "forgot password" affordance is hidden
/// in the UI (see `get_platform`) and this returns false.
#[cfg(target_os = "linux")]
pub fn reset_with_admin() -> bool {
    std::process::Command::new("pkexec")
        .arg("/bin/true")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
pub fn reset_with_admin() -> bool {
    false
}
