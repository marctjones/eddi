// Handshake and authentication utilities for broker/client introduction

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Introduction handshake data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntroductionData {
    /// Fortress onion address
    pub fortress_address: String,
    /// Access token for fortress
    pub access_token: String,
    /// Token expiration time
    pub expires_at: SystemTime,
}

/// Generate a short code for broker discovery
/// Format: XXX-YYY (6 characters total)
pub fn generate_short_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ"; // No confusing chars (0,O,1,I)
    const CODE_LEN: usize = 6;

    let mut rng = rand::thread_rng();
    let code: String = (0..CODE_LEN)
        .map(|i| {
            let idx = rng.gen_range(0..CHARSET.len());
            let c = CHARSET[idx] as char;
            if i == 3 {
                format!("-{}", c)
            } else {
                c.to_string()
            }
        })
        .collect();

    code
}

/// Generate access token for fortress
pub fn generate_access_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const TOKEN_LEN: usize = 32;

    let mut rng = rand::thread_rng();
    (0..TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Generate deterministic onion service identifier for broker
/// Based on namespace (email/identifier) + timestamp + short code
pub fn generate_broker_identifier(namespace: &str, timestamp: u64, code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update(timestamp.to_le_bytes());
    hasher.update(code.as_bytes());

    let result = hasher.finalize();
    hex::encode(&result[..16]) // Use first 16 bytes for identifier
}

/// Get current timestamp in seconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Round timestamp to nearest interval (for time-based discovery)
pub fn round_timestamp(timestamp: u64, interval_seconds: u64) -> u64 {
    (timestamp / interval_seconds) * interval_seconds
}

/// Generate time window for broker discovery
/// Returns timestamps to try (±window from current time)
pub fn generate_time_window(window_minutes: i64) -> Vec<u64> {
    const INTERVAL_SECONDS: u64 = 60; // 1 minute intervals

    let now = current_timestamp();
    let rounded_now = round_timestamp(now, INTERVAL_SECONDS);

    let window_seconds = (window_minutes * 60) as u64;
    let num_intervals = (window_seconds / INTERVAL_SECONDS) * 2 + 1; // ±window plus center

    let mut timestamps = Vec::new();

    for i in 0..num_intervals {
        let offset = (i as i64 - (num_intervals / 2) as i64) * INTERVAL_SECONDS as i64;
        let timestamp = (rounded_now as i64 + offset) as u64;
        timestamps.push(round_timestamp(timestamp, INTERVAL_SECONDS));
    }

    timestamps
}

/// Broker handshake handler
pub struct BrokerHandshake {
    namespace: String,
    code: String,
    timestamp: u64,
    fortress_address: String,
}

impl BrokerHandshake {
    /// Create a new broker handshake
    pub fn new(namespace: String, code: String, fortress_address: String) -> Self {
        let timestamp = round_timestamp(current_timestamp(), 60);

        Self {
            namespace,
            code,
            timestamp,
            fortress_address,
        }
    }

    /// Get the broker identifier
    pub fn identifier(&self) -> String {
        generate_broker_identifier(&self.namespace, self.timestamp, &self.code)
    }

    /// Create introduction data for a client
    pub fn create_introduction(&self, token_ttl_hours: u64) -> IntroductionData {
        let access_token = generate_access_token();
        let expires_at = SystemTime::now() + std::time::Duration::from_secs(token_ttl_hours * 3600);

        IntroductionData {
            fortress_address: self.fortress_address.clone(),
            access_token,
            expires_at,
        }
    }

    /// Validate a client's connection attempt
    pub fn validate_code(&self, provided_code: &str) -> bool {
        self.code == provided_code
    }
}

/// Client handshake handler
pub struct ClientHandshake {
    namespace: String,
    code: String,
}

impl ClientHandshake {
    /// Create a new client handshake
    pub fn new(namespace: String, code: String) -> Self {
        Self { namespace, code }
    }

    /// Generate possible broker identifiers to try
    pub fn possible_identifiers(&self, time_window_minutes: i64) -> Vec<(u64, String)> {
        let timestamps = generate_time_window(time_window_minutes);

        timestamps
            .into_iter()
            .map(|ts| {
                let identifier = generate_broker_identifier(&self.namespace, ts, &self.code);
                (ts, identifier)
            })
            .collect()
    }

    /// Get the code
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Get the namespace
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_short_code() {
        let code = generate_short_code();
        assert_eq!(code.len(), 7); // 6 chars + 1 hyphen
        assert!(code.contains('-'));

        // Should only contain valid characters
        for c in code.chars() {
            if c != '-' {
                assert!("23456789ABCDEFGHJKLMNPQRSTUVWXYZ".contains(c));
            }
        }
    }

    #[test]
    fn test_generate_access_token() {
        let token = generate_access_token();
        assert_eq!(token.len(), 32);

        // Should only contain alphanumeric characters
        for c in token.chars() {
            assert!(c.is_alphanumeric());
        }
    }

    #[test]
    fn test_broker_identifier_deterministic() {
        let namespace = "test@example.com";
        let timestamp = 1234567890;
        let code = "ABC-XYZ";

        let id1 = generate_broker_identifier(namespace, timestamp, code);
        let id2 = generate_broker_identifier(namespace, timestamp, code);

        assert_eq!(id1, id2, "Identifiers should be deterministic");
    }

    #[test]
    fn test_broker_identifier_unique() {
        let namespace = "test@example.com";
        let timestamp = 1234567890;

        let id1 = generate_broker_identifier(namespace, timestamp, "ABC-XYZ");
        let id2 = generate_broker_identifier(namespace, timestamp, "DEF-123");

        assert_ne!(id1, id2, "Different codes should produce different identifiers");
    }

    #[test]
    fn test_round_timestamp() {
        let ts1 = 1234567890; // Some timestamp
        let rounded = round_timestamp(ts1, 60);

        assert_eq!(rounded % 60, 0, "Should be divisible by interval");
        assert!(rounded <= ts1, "Should round down");
        assert!(ts1 - rounded < 60, "Should be within one interval");
    }

    #[test]
    fn test_time_window() {
        let window = generate_time_window(2); // ±2 minutes

        // Should have at least 5 timestamps (±2 minutes at 1-minute intervals)
        assert!(window.len() >= 5);

        // All timestamps should be rounded to minute boundaries
        for ts in &window {
            assert_eq!(ts % 60, 0);
        }

        // Timestamps should be sorted
        for i in 1..window.len() {
            assert!(window[i] >= window[i-1]);
        }
    }

    #[test]
    fn test_broker_handshake() {
        let handshake = BrokerHandshake::new(
            "test@example.com".to_string(),
            "ABC-XYZ".to_string(),
            "test123.onion".to_string(),
        );

        let identifier = handshake.identifier();
        assert!(!identifier.is_empty());

        assert!(handshake.validate_code("ABC-XYZ"));
        assert!(!handshake.validate_code("WRONG"));
    }

    #[test]
    fn test_client_handshake() {
        let handshake = ClientHandshake::new(
            "test@example.com".to_string(),
            "ABC-XYZ".to_string(),
        );

        let identifiers = handshake.possible_identifiers(2);

        assert!(identifiers.len() >= 5);

        // All identifiers should be hex strings
        for (_, id) in &identifiers {
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_introduction_data() {
        let handshake = BrokerHandshake::new(
            "test@example.com".to_string(),
            "ABC-XYZ".to_string(),
            "test123.onion".to_string(),
        );

        let intro = handshake.create_introduction(24);

        assert_eq!(intro.fortress_address, "test123.onion");
        assert!(!intro.access_token.is_empty());
        assert!(intro.expires_at > SystemTime::now());
    }
}
