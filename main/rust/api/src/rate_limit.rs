//! IP-based rate limiting for invite generation
//!
//! Stores rate limit data in a JSON file, allowing persistence across restarts.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

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
    /// Map of IP address string to last invite timestamp (RFC 3339)
    pub invites: HashMap<String, String>,
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
        let _guard = self.lock.lock().map_err(|_| RateLimitError::Lock)?;

        let mut data = self.load()?;
        let ip_str = ip.to_string();
        let now = Utc::now();

        // Check if IP has a recent invite
        if let Some(last_invite) = data.invites.get(&ip_str) {
            if let Ok(last_time) = DateTime::parse_from_rfc3339(last_invite) {
                let last_time_utc: DateTime<Utc> = last_time.into();
                if now - last_time_utc < Duration::hours(self.window_hours) {
                    return Ok(false); // Rate limited
                }
            }
        }

        // Clean up old entries (older than window)
        let window = Duration::hours(self.window_hours);
        data.invites.retain(|_, v| {
            if let Ok(t) = DateTime::parse_from_rfc3339(v) {
                let t_utc: DateTime<Utc> = t.into();
                now - t_utc < window
            } else {
                false
            }
        });

        // Record new invite
        data.invites.insert(ip_str, now.to_rfc3339());
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

        if let Some(last_invite) = data.invites.get(&ip_str) {
            if let Ok(last_time) = DateTime::parse_from_rfc3339(last_invite) {
                let last_time_utc: DateTime<Utc> = last_time.into();
                let elapsed = now - last_time_utc;
                let window = Duration::hours(self.window_hours);

                if elapsed < window {
                    let remaining = window - elapsed;
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
    fn test_rate_limiter_blocks_second_request() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert!(limiter.check_and_record(ip).unwrap());
        assert!(!limiter.check_and_record(ip).unwrap());
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        assert!(limiter.check_and_record(ip1).unwrap());
        assert!(limiter.check_and_record(ip2).unwrap());
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

        // After request, should have retry time
        limiter.check_and_record(ip).unwrap();
        let retry = limiter.get_retry_after(ip).unwrap();
        assert!(retry.is_some());
        assert!(retry.unwrap() > 0);
    }
}
