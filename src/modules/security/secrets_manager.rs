//! Secrets validation and sanitized logging
//!
//! Ensures critical secrets are present and never logged in plaintext.

use std::fmt;

/// Wrapper for secret strings that redacts on Debug/Display
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    /// Create a new secret string
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Get the actual secret value (use sparingly!)
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Check if the secret is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Validates required environment variables for production
pub struct SecretValidator;

impl SecretValidator {
    /// Validate that all required secrets are present and non-empty
    /// Panics with clear error message if validation fails
    pub fn validate_required_secrets() {
        let required_vars = vec![
            ("CTRADER_CLIENT_ID", "cTrader OAuth client ID"),
            ("CTRADER_CLIENT_SECRET", "cTrader OAuth client secret"),
            ("CTRADER_ACCOUNT_ID", "cTrader account ID"),
            ("PERPLEXITY_API_KEY", "Perplexity API key for sentiment analysis"),
        ];

        let mut missing = Vec::new();
        let mut empty = Vec::new();

        for (var_name, description) in &required_vars {
            match std::env::var(var_name) {
                Ok(value) if !value.is_empty() => {
                    // Valid
                }
                Ok(_) => {
                    empty.push(format!("  - {} ({})", var_name, description));
                }
                Err(_) => {
                    missing.push(format!("  - {} ({})", var_name, description));
                }
            }
        }

        if !missing.is_empty() || !empty.is_empty() {
            let mut error_msg = String::from("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            error_msg.push_str("⚠️  SECURITY ERROR: Missing or empty required environment variables\n");
            error_msg.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

            if !missing.is_empty() {
                error_msg.push_str("Missing variables:\n");
                error_msg.push_str(&missing.join("\n"));
                error_msg.push('\n');
            }

            if !empty.is_empty() {
                if !missing.is_empty() {
                    error_msg.push('\n');
                }
                error_msg.push_str("Empty variables:\n");
                error_msg.push_str(&empty.join("\n"));
                error_msg.push('\n');
            }

            error_msg.push_str("\n📋 How to fix:\n");
            error_msg.push_str("  1. Copy .env.example to .env\n");
            error_msg.push_str("  2. Fill in your credentials from cTrader and Perplexity\n");
            error_msg.push_str("  3. For CTRADER_ACCESS_TOKEN, run: cargo run --bin get-token\n");
            error_msg.push_str("  4. Restart the bot\n\n");
            error_msg.push_str("For Railway deployment, set these as environment variables in your Railway project.\n");
            error_msg.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            panic!("{}", error_msg);
        }
    }

    /// Validate optional OAuth access token (warn if missing)
    pub fn validate_access_token() -> Option<String> {
        match std::env::var("CTRADER_ACCESS_TOKEN") {
            Ok(token) if !token.is_empty() => Some(token),
            _ => {
                tracing::warn!(
                    "CTRADER_ACCESS_TOKEN not set. OAuth flow required. Run: cargo run --bin get-token"
                );
                None
            }
        }
    }

    /// Sanitize a string for safe logging (truncate and redact middle)
    pub fn sanitize_for_logging(secret: &str, prefix_len: usize, suffix_len: usize) -> String {
        if secret.len() <= prefix_len + suffix_len {
            return "[REDACTED]".to_string();
        }

        let prefix = &secret[..prefix_len];
        let suffix = &secret[secret.len() - suffix_len..];
        let redacted_len = secret.len() - prefix_len - suffix_len;

        format!("{}***({} chars)***{}", prefix, redacted_len, suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner())
    }

    #[test]
    fn test_secret_string_redacts_debug() {
        let secret = SecretString::new("my-secret-key-12345".to_string());
        let debug_output = format!("{:?}", secret);
        assert_eq!(debug_output, "[REDACTED]");
    }

    #[test]
    fn test_secret_string_redacts_display() {
        let secret = SecretString::new("my-secret-key-12345".to_string());
        let display_output = format!("{}", secret);
        assert_eq!(display_output, "[REDACTED]");
    }

    #[test]
    fn test_secret_string_expose() {
        let secret = SecretString::new("actual-value".to_string());
        assert_eq!(secret.expose_secret(), "actual-value");
    }

    #[test]
    fn test_secret_string_is_empty() {
        let empty = SecretString::new(String::new());
        let non_empty = SecretString::new("value".to_string());
        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_sanitize_short_string() {
        let sanitized = SecretValidator::sanitize_for_logging("abc", 2, 2);
        assert_eq!(sanitized, "[REDACTED]");
    }

    #[test]
    fn test_sanitize_long_string() {
        let sanitized = SecretValidator::sanitize_for_logging("abcdefghijklmnop", 3, 3);
        assert_eq!(sanitized, "abc***(10 chars)***nop");
    }

    #[test]
    fn test_validate_access_token_present() {
        let _lock = lock_env();
        env::set_var("CTRADER_ACCESS_TOKEN", "test-token");
        let token = SecretValidator::validate_access_token();
        assert_eq!(token, Some("test-token".to_string()));
        env::remove_var("CTRADER_ACCESS_TOKEN");
    }

    #[test]
    fn test_validate_access_token_missing() {
        let _lock = lock_env();
        env::remove_var("CTRADER_ACCESS_TOKEN");
        let token = SecretValidator::validate_access_token();
        assert_eq!(token, None);
    }

    #[test]
    #[should_panic(expected = "SECURITY ERROR")]
    fn test_validate_required_secrets_missing() {
        let _lock = lock_env();
        // Clear all required vars
        env::remove_var("CTRADER_CLIENT_ID");
        env::remove_var("CTRADER_CLIENT_SECRET");
        env::remove_var("CTRADER_ACCOUNT_ID");
        env::remove_var("PERPLEXITY_API_KEY");

        SecretValidator::validate_required_secrets();
    }
}
