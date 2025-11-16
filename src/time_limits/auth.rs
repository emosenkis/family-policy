use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Admin authentication handler
pub struct AdminAuth;

impl AdminAuth {
    /// Hash a password using Argon2id
    pub fn hash_password(password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();

        Ok(password_hash)
    }

    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Failed to parse password hash: {}", e))?;

        let argon2 = Argon2::default();

        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Check if the current OS user is an admin account
    pub fn is_admin_account(username: &str, admin_accounts: &[String]) -> bool {
        admin_accounts.iter().any(|admin| admin == username)
    }

    /// Get the current OS username
    pub fn get_current_username() -> Result<String> {
        #[cfg(target_os = "windows")]
        {
            Ok(std::env::var("USERNAME")
                .context("Failed to get USERNAME environment variable")?)
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(std::env::var("USER")
                .context("Failed to get USER environment variable")?)
        }
    }
}

/// Rate limiter for password attempts
pub struct RateLimiter {
    attempts: Vec<std::time::Instant>,
    max_attempts: usize,
    window_duration: std::time::Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window_seconds: u64) -> Self {
        Self {
            attempts: Vec::new(),
            max_attempts,
            window_duration: std::time::Duration::from_secs(window_seconds),
        }
    }

    /// Check if an attempt is allowed
    pub fn is_allowed(&mut self) -> bool {
        let now = std::time::Instant::now();

        // Remove old attempts outside the window
        self.attempts.retain(|&attempt| {
            now.duration_since(attempt) < self.window_duration
        });

        // Check if we're under the limit
        if self.attempts.len() < self.max_attempts {
            self.attempts.push(now);
            true
        } else {
            false
        }
    }

    /// Get time until next attempt is allowed
    pub fn time_until_allowed(&self) -> Option<std::time::Duration> {
        if self.attempts.len() < self.max_attempts {
            return None;
        }

        let oldest = self.attempts.first()?;
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(*oldest);

        if elapsed < self.window_duration {
            Some(self.window_duration - elapsed)
        } else {
            None
        }
    }

    /// Reset the rate limiter
    pub fn reset(&mut self) {
        self.attempts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "test_password_123";
        let hash = AdminAuth::hash_password(password).unwrap();

        assert!(AdminAuth::verify_password(password, &hash).unwrap());
        assert!(!AdminAuth::verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_hash_is_different_each_time() {
        let password = "test_password";
        let hash1 = AdminAuth::hash_password(password).unwrap();
        let hash2 = AdminAuth::hash_password(password).unwrap();

        // Hashes should be different due to different salts
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(AdminAuth::verify_password(password, &hash1).unwrap());
        assert!(AdminAuth::verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_is_admin_account() {
        let admin_accounts = vec!["admin".to_string(), "parent".to_string()];

        assert!(AdminAuth::is_admin_account("admin", &admin_accounts));
        assert!(AdminAuth::is_admin_account("parent", &admin_accounts));
        assert!(!AdminAuth::is_admin_account("child", &admin_accounts));
    }

    #[test]
    fn test_rate_limiter_allows_attempts() {
        let mut limiter = RateLimiter::new(3, 60);

        assert!(limiter.is_allowed());
        assert!(limiter.is_allowed());
        assert!(limiter.is_allowed());
        assert!(!limiter.is_allowed()); // 4th attempt should fail
    }

    #[test]
    fn test_rate_limiter_reset() {
        let mut limiter = RateLimiter::new(2, 60);

        assert!(limiter.is_allowed());
        assert!(limiter.is_allowed());
        assert!(!limiter.is_allowed());

        limiter.reset();
        assert!(limiter.is_allowed()); // Should work after reset
    }

    #[test]
    fn test_rate_limiter_window_expiry() {
        let mut limiter = RateLimiter::new(2, 1); // 1 second window

        assert!(limiter.is_allowed());
        assert!(limiter.is_allowed());
        assert!(!limiter.is_allowed());

        // Wait for window to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        assert!(limiter.is_allowed()); // Should work after window expires
    }
}
