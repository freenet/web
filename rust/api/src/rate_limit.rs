//! IP-based rate limiting for invite generation
//!
//! Stores rate limit data in a JSON file, allowing persistence across restarts.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

/// Maximum number of invites allowed per IP within the time window
const MAX_INVITES_PER_WINDOW: usize = 20;

/// SHA256 hashes of IPs exempt from rate limiting (for testing)
const EXEMPT_IP_HASHES: &[&str] = &[
    "0cf75236cce089f9c592bb2b50925c48cbbb4d0f83094b2cd091dda4b53e1a4c",
];

/// Check if an IP is exempt from rate limiting
fn is_exempt(ip: &IpAddr) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(ip.to_string().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    EXEMPT_IP_HASHES.contains(&hash.as_str())
}

#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Lock error")]
    Lock,
}

/// Stored rate limit data
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct RateLimitData {
    /// Map of IP address string to list of invite timestamps (RFC 3339)
    pub invites: HashMap<String, Vec<String>>,
}

/// Rate limiter with file-based persistence
pub struct RateLimiter {
    data_path: PathBuf,
    window_hours: i64,
    /// Mutex for thread-safe access to the file
    lock: Mutex<()>,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `data_path` - Path to the JSON file for persistence
    /// * `window_hours` - Time window in hours (e.g., 24 for once per day)
    pub fn new(data_path: PathBuf, window_hours: i64) -> Self {
        Self {
            data_path,
            window_hours,
            lock: Mutex::new(()),
        }
    }

    /// Check if an IP is rate limited, and record the access if allowed
    ///
    /// Returns Ok(true) if the request is allowed, Ok(false) if rate limited
    pub fn check_and_record(&self, ip: IpAddr) -> Result<bool, RateLimitError> {
        // Check exemption first (before acquiring lock)
        if is_exempt(&ip) {
            return Ok(true);
        }

        let _guard = self.lock.lock().map_err(|_| RateLimitError::Lock)?;

        let mut data = self.load()?;
        let ip_str = ip.to_string();
        let now = Utc::now();
        let window = Duration::hours(self.window_hours);

        // Clean up old entries for all IPs
        for timestamps in data.invites.values_mut() {
            timestamps.retain(|ts| {
                if let Ok(t) = DateTime::parse_from_rfc3339(ts) {
                    let t_utc: DateTime<Utc> = t.into();
                    now - t_utc < window
                } else {
                    false
                }
            });
        }
        // Remove IPs with no remaining timestamps
        data.invites.retain(|_, v| !v.is_empty());

        // Check if IP has reached the limit
        let timestamps = data.invites.entry(ip_str).or_default();
        if timestamps.len() >= MAX_INVITES_PER_WINDOW {
            return Ok(false); // Rate limited
        }

        // Record new invite
        timestamps.push(now.to_rfc3339());
        self.save(&data)?;

        Ok(true)
    }

    /// Get the remaining time until an IP can request again
    ///
    /// Returns None if the IP is not rate limited, Some(seconds) otherwise
    pub fn get_retry_after(&self, ip: IpAddr) -> Result<Option<i64>, RateLimitError> {
        let _guard = self.lock.lock().map_err(|_| RateLimitError::Lock)?;

        let data = self.load()?;
        let ip_str = ip.to_string();
        let now = Utc::now();
        let window = Duration::hours(self.window_hours);

        if let Some(timestamps) = data.invites.get(&ip_str) {
            // Filter to only valid timestamps within window
            let valid_timestamps: Vec<_> = timestamps
                .iter()
                .filter_map(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|t| -> DateTime<Utc> { t.into() })
                .filter(|t| now - *t < window)
                .collect();

            // If at limit, return time until oldest expires
            if valid_timestamps.len() >= MAX_INVITES_PER_WINDOW {
                if let Some(oldest) = valid_timestamps.iter().min() {
                    let expires_at = *oldest + window;
                    let remaining = expires_at - now;
                    return Ok(Some(remaining.num_seconds()));
                }
            }
        }

        Ok(None)
    }

    fn load(&self) -> Result<RateLimitData, RateLimitError> {
        if self.data_path.exists() {
            let content = fs::read_to_string(&self.data_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(RateLimitData::default())
        }
    }

    fn save(&self, data: &RateLimitData) -> Result<(), RateLimitError> {
        // Ensure parent directory exists
        if let Some(parent) = self.data_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(data)?;
        fs::write(&self.data_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tempfile::tempdir;

    #[test]
    fn test_rate_limiter_allows_first_request() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert!(limiter.check_and_record(ip).unwrap());
    }

    #[test]
    fn test_rate_limiter_allows_up_to_max_requests() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Should allow MAX_INVITES_PER_WINDOW requests
        for i in 0..MAX_INVITES_PER_WINDOW {
            assert!(limiter.check_and_record(ip).unwrap(), "Request {} should be allowed", i + 1);
        }

        // Next request should be blocked
        assert!(!limiter.check_and_record(ip).unwrap(), "Request {} should be blocked", MAX_INVITES_PER_WINDOW + 1);
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        // Both IPs should be able to use their full quota
        for _ in 0..MAX_INVITES_PER_WINDOW {
            assert!(limiter.check_and_record(ip1).unwrap());
            assert!(limiter.check_and_record(ip2).unwrap());
        }

        // Both should now be blocked
        assert!(!limiter.check_and_record(ip1).unwrap());
        assert!(!limiter.check_and_record(ip2).unwrap());
    }

    #[test]
    fn test_get_retry_after() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Before any request, no retry needed
        assert!(limiter.get_retry_after(ip).unwrap().is_none());

        // After using quota, no retry yet (still have remaining)
        limiter.check_and_record(ip).unwrap();
        assert!(limiter.get_retry_after(ip).unwrap().is_none());

        // Use up remaining quota
        for _ in 1..MAX_INVITES_PER_WINDOW {
            limiter.check_and_record(ip).unwrap();
        }

        // Now should have retry time
        let retry = limiter.get_retry_after(ip).unwrap();
        assert!(retry.is_some());
        assert!(retry.unwrap() > 0);
    }
}
