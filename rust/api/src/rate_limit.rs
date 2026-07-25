//! IP-based rate limiting for invite generation
//!
//! Stores rate limit data in a JSON file, allowing persistence across restarts.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

/// Maximum number of invites allowed per IP within the time window.
///
/// Kept deliberately low as an anti-spam measure for the freenet.org/quickstart
/// flow, which mints invites into the shared "Freenet Official" room. A few
/// invites per day covers legitimate re-invites while still slowing bulk
/// account creation. Raising this without a matching anti-abuse story re-opens
/// the vector this limit exists to slow. See
/// `test_rate_limiter_enforces_four_per_window`.
///
/// NOTE: this limit is per-IP and therefore cannot bound an actor who rotates
/// IPs. For the Tor exit set specifically, see [`AggregateBucket`] and
/// [`TOR_INVITES_PER_HOUR`].
pub const MAX_INVITES_PER_WINDOW: usize = 4;

/// Shared hourly invite ceiling across the ENTIRE Tor exit set.
///
/// Sizing (measured 2026-07-24/25 from `invite_rate_limits.json`): organic Tor
/// traffic peaked at **11 invites/hour** (mean 6.8) across 6 non-burst hours,
/// while the abuse burst hit **158/hour**. 25 leaves 2.3x headroom over the
/// worst observed organic hour, so ordinary Tor users are unaffected, while
/// capping a rotation burst at ~16% of what it achieved. See
/// `tor_bucket_admits_organic_peak_but_caps_burst`.
pub const TOR_INVITES_PER_HOUR: usize = 25;

/// Window for [`TOR_INVITES_PER_HOUR`], in minutes.
pub const TOR_WINDOW_MINUTES: i64 = 60;

/// SHA256 hashes of IPs exempt from rate limiting (for testing)
const EXEMPT_IP_HASHES: &[&str] =
    &["0cf75236cce089f9c592bb2b50925c48cbbb4d0f83094b2cd091dda4b53e1a4c"];

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

/// A single sliding-window counter shared by many identities.
///
/// # Why a shared bucket
///
/// Per-IP limiting assumes an IP approximates a person. For Tor that assumption
/// is false in the attacker's favour: exits are a public, rotatable pool, so N
/// exits multiply any per-IP limit by N. Metering the whole pool through ONE
/// bucket removes the multiplier entirely — rotating costs the attacker nothing
/// and gains them nothing.
///
/// Deliberately in-memory (not persisted like [`RateLimiter`]): a restart
/// forgives at most [`TOR_INVITES_PER_HOUR`] requests, gkapi restarts only on
/// deploy, and this keeps the hot path free of the read-modify-write file IO
/// the per-IP limiter does.
pub struct AggregateBucket {
    limit: usize,
    window: Duration,
    hits: Mutex<VecDeque<DateTime<Utc>>>,
}

impl AggregateBucket {
    pub fn new(limit: usize, window_minutes: i64) -> Self {
        Self {
            limit,
            window: Duration::minutes(window_minutes),
            hits: Mutex::new(VecDeque::new()),
        }
    }

    /// Drop hits that have aged out of the window.
    fn prune(&self, hits: &mut VecDeque<DateTime<Utc>>, now: DateTime<Utc>) {
        while let Some(front) = hits.front() {
            if now - *front >= self.window {
                hits.pop_front();
            } else {
                break;
            }
        }
    }

    /// Is there room in the window right now?
    ///
    /// Checked separately from [`Self::record`] so a caller can reject a request
    /// BEFORE spending the requester's per-IP allowance on it (see the ordering
    /// note in `routes::create_room_invite`). The gap between the two admits a
    /// small overshoot under concurrent load, bounded by the number of requests
    /// in flight; for an anti-abuse ceiling that is not worth a global lock.
    pub fn has_capacity(&self) -> bool {
        let now = Utc::now();
        match self.hits.lock() {
            Ok(mut hits) => {
                self.prune(&mut hits, now);
                hits.len() < self.limit
            }
            // Fail open: a poisoned lock must not block legitimate users.
            Err(_) => true,
        }
    }

    /// Record one hit against the window.
    pub fn record(&self) {
        let now = Utc::now();
        if let Ok(mut hits) = self.hits.lock() {
            self.prune(&mut hits, now);
            hits.push_back(now);
        }
    }

    /// Seconds until the window has room again, or `None` if it has room now.
    pub fn retry_after_seconds(&self) -> Option<i64> {
        let now = Utc::now();
        let mut hits = self.hits.lock().ok()?;
        self.prune(&mut hits, now);
        if hits.len() < self.limit {
            return None;
        }
        hits.front()
            .map(|oldest| (*oldest + self.window - now).num_seconds().max(0))
    }

    /// Current occupancy (for logging / diagnostics).
    pub fn current(&self) -> usize {
        let now = Utc::now();
        match self.hits.lock() {
            Ok(mut hits) => {
                self.prune(&mut hits, now);
                hits.len()
            }
            Err(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tempfile::tempdir;

    #[test]
    fn aggregate_bucket_admits_up_to_limit_then_refuses() {
        let bucket = AggregateBucket::new(3, 60);
        for i in 1..=3 {
            assert!(bucket.has_capacity(), "hit {i} should have capacity");
            bucket.record();
        }
        assert!(!bucket.has_capacity(), "4th hit must be refused");
        assert_eq!(bucket.current(), 3);
        assert!(bucket.retry_after_seconds().is_some());
    }

    /// The whole point: many distinct identities share ONE budget, so rotating
    /// between them gains nothing.
    #[test]
    fn aggregate_bucket_is_not_per_identity() {
        let bucket = AggregateBucket::new(2, 60);
        // Simulate three different "IPs" all funnelling through one bucket.
        bucket.record(); // exit A
        bucket.record(); // exit B
        assert!(
            !bucket.has_capacity(),
            "a third distinct exit must NOT get its own allowance"
        );
    }

    /// Sizing regression: the deployed ceiling must stay above the measured
    /// organic Tor peak (11/hour) and far below the observed burst (158/hour).
    #[test]
    fn tor_bucket_admits_organic_peak_but_caps_burst() {
        const OBSERVED_ORGANIC_PEAK: usize = 11;
        const OBSERVED_BURST: usize = 158;

        assert!(
            TOR_INVITES_PER_HOUR > OBSERVED_ORGANIC_PEAK,
            "ceiling {TOR_INVITES_PER_HOUR} must exceed the organic peak \
             {OBSERVED_ORGANIC_PEAK}/h or real Tor users get blocked"
        );
        assert!(
            TOR_INVITES_PER_HOUR < OBSERVED_BURST / 2,
            "ceiling {TOR_INVITES_PER_HOUR} must be well under the observed \
             burst {OBSERVED_BURST}/h or it does not actually bound abuse"
        );

        let bucket = AggregateBucket::new(TOR_INVITES_PER_HOUR, TOR_WINDOW_MINUTES);
        for _ in 0..OBSERVED_ORGANIC_PEAK {
            assert!(
                bucket.has_capacity(),
                "organic traffic must never be refused"
            );
            bucket.record();
        }
        // Now replay the burst; it must be cut off at the ceiling.
        let mut admitted = OBSERVED_ORGANIC_PEAK;
        for _ in 0..OBSERVED_BURST {
            if bucket.has_capacity() {
                bucket.record();
                admitted += 1;
            }
        }
        assert_eq!(
            admitted, TOR_INVITES_PER_HOUR,
            "burst must be capped at the ceiling, not merely slowed"
        );
    }

    #[test]
    fn aggregate_bucket_expires_old_hits() {
        let bucket = AggregateBucket::new(2, 60);
        // Backdate both hits beyond the window.
        {
            let mut hits = bucket.hits.lock().unwrap();
            let old = Utc::now() - Duration::minutes(61);
            hits.push_back(old);
            hits.push_back(old);
        }
        assert_eq!(bucket.current(), 0, "expired hits must be pruned");
        assert!(bucket.has_capacity());
        assert!(bucket.retry_after_seconds().is_none());
    }

    #[test]
    fn test_rate_limiter_allows_first_request() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert!(limiter.check_and_record(ip).unwrap());
    }

    /// Pins the deployed anti-spam limit at exactly 4 invites / IP / window.
    ///
    /// The other tests are parameterized on `MAX_INVITES_PER_WINDOW`, so they
    /// pass at any value and would NOT catch an accidental bump of the
    /// constant. This one hardcodes the intended value: the 5th invite from the
    /// same IP inside the window must be rejected.
    #[test]
    fn test_rate_limiter_enforces_four_per_window() {
        assert_eq!(
            MAX_INVITES_PER_WINDOW, 4,
            "invite anti-spam limit must stay at 4/IP/window; raising it re-opens bulk account creation"
        );

        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        for i in 1..=4 {
            assert!(
                limiter.check_and_record(ip).unwrap(),
                "invite {i} of 4 should be allowed"
            );
        }
        assert!(
            !limiter.check_and_record(ip).unwrap(),
            "5th invite in window must be rejected"
        );
    }

    #[test]
    fn test_rate_limiter_allows_up_to_max_requests() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rate_limits.json");
        let limiter = RateLimiter::new(path, 24);

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // Should allow MAX_INVITES_PER_WINDOW requests
        for i in 0..MAX_INVITES_PER_WINDOW {
            assert!(
                limiter.check_and_record(ip).unwrap(),
                "Request {} should be allowed",
                i + 1
            );
        }

        // Next request should be blocked
        assert!(
            !limiter.check_and_record(ip).unwrap(),
            "Request {} should be blocked",
            MAX_INVITES_PER_WINDOW + 1
        );
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
