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
use std::time::{Duration as StdDuration, Instant};
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

/// Default shared hourly invite ceiling across the ENTIRE Tor exit set.
///
/// # Sizing, and why this number is uncomfortable
///
/// From `invite_rate_limits.json` (1728 invites, 2026-07-24T06:23Z..
/// 2026-07-25T05:54Z), classified against the official bulk exit list:
///
/// | Tor invites/hour | |
/// |---|---|
/// | organic hours (burst window excluded BY TIME, n=7) | mean 11.9, **max 33** |
/// | burst hour 02:00Z | **208** |
///
/// An earlier revision used 25, derived from a smaller sample filtered by
/// MAGNITUDE (hours above a threshold were excluded as "burst", then the
/// remaining max was called the organic peak). That is circular, and it
/// understated the peak by 3x. 25 would have refused real users.
///
/// 60 sits above the observed organic max (33) with ~1.8x headroom and still
/// cuts the observed burst hour by ~71%. It is a judgement call on ambiguous
/// data, not a derived constant: hours 04Z (28 invites / 25 exits) and 05Z
/// (33 / 33) are ~1.0 invites per exit, which looks like many distinct real
/// users -- but a one-request-per-exit attacker is indistinguishable from that
/// by volume alone. Treat this as a starting value to tune from telemetry,
/// which is why it is overridable at runtime (`--tor-invites-per-hour` /
/// `TOR_INVITES_PER_HOUR`, `0` disables the ceiling entirely).
pub const DEFAULT_TOR_INVITES_PER_HOUR: usize = 60;

/// Window for the Tor ceiling, in minutes.
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
/// bucket removes the multiplier.
///
/// # The cost, stated plainly
///
/// Sharing a budget across an anonymity set means the loudest member can
/// consume the whole allowance. An attacker who sustains the ceiling denies
/// invites to EVERY Tor user for as long as they keep it up, and refused
/// requests cost them nothing, so an automated poller wins each freed slot
/// against a human. This is inherent to any aggregate cap over an unlinkable
/// pool, not an implementation defect -- fixing it needs a per-request cost
/// signal (proof-of-work / CAPTCHA), tracked in freenet/web#81. Do not describe
/// this bucket as leaving ordinary Tor users unaffected: that is only true
/// while nobody is attacking.
///
/// Deliberately in-memory (not persisted like [`RateLimiter`]): a restart
/// forgives at most one window's worth, gkapi restarts only on deploy, and this
/// keeps the hot path free of the read-modify-write file IO the per-IP limiter
/// does.
///
/// Uses a MONOTONIC clock ([`Instant`]) rather than wall time: an NTP step
/// backwards would otherwise stall pruning and pin the bucket at full, denying
/// every Tor user until the clock caught up. [`RateLimiter`] genuinely needs
/// wall time because it persists; this does not.
pub struct AggregateBucket {
    limit: usize,
    window: StdDuration,
    hits: Mutex<VecDeque<Instant>>,
}

impl AggregateBucket {
    pub fn new(limit: usize, window_minutes: i64) -> Self {
        Self {
            limit,
            window: StdDuration::from_secs((window_minutes.max(0) as u64) * 60),
            hits: Mutex::new(VecDeque::new()),
        }
    }

    /// A limit of 0 disables the ceiling entirely (runtime off-switch).
    pub fn is_disabled(&self) -> bool {
        self.limit == 0
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Drop hits that have aged out of the window.
    ///
    /// `hits` is append-only with the clock sampled UNDER the lock, so it is
    /// sorted ascending and the front is always the oldest.
    fn prune(&self, hits: &mut VecDeque<Instant>, now: Instant) {
        while let Some(front) = hits.front() {
            if now.duration_since(*front) >= self.window {
                hits.pop_front();
            } else {
                break;
            }
        }
    }

    /// Atomically take a slot if one is free.
    ///
    /// This is the ADMISSION AUTHORITY -- check and record happen under a
    /// single lock acquisition, so the ceiling holds no matter how many
    /// requests race, and stays correct if an `.await` is ever introduced
    /// between the pre-check and here. Returns false when the window is full.
    pub fn try_acquire(&self) -> bool {
        if self.is_disabled() {
            return true;
        }
        let now = Instant::now();
        match self.hits.lock() {
            Ok(mut hits) => {
                self.prune(&mut hits, now);
                if hits.len() < self.limit {
                    hits.push_back(now);
                    true
                } else {
                    false
                }
            }
            // Fail open: a poisoned lock must not block legitimate users.
            Err(_) => true,
        }
    }

    /// Give back a slot taken by [`Self::try_acquire`] for a request that was
    /// not ultimately served.
    ///
    /// Removes the newest entry rather than the specific one acquired. Since
    /// every entry in flight is within microseconds of the others and only the
    /// COUNT is meaningful, that is equivalent, and it avoids handing out
    /// tokens just to identify which entry to drop.
    pub fn release(&self) {
        if self.is_disabled() {
            return;
        }
        if let Ok(mut hits) = self.hits.lock() {
            hits.pop_back();
        }
    }

    /// Cheap non-consuming pre-check.
    ///
    /// Used only to reject early WITHOUT spending the requester's per-IP
    /// allowance. It is advisory: [`Self::try_acquire`] is what actually
    /// enforces the ceiling.
    pub fn has_capacity(&self) -> bool {
        if self.is_disabled() {
            return true;
        }
        let now = Instant::now();
        match self.hits.lock() {
            Ok(mut hits) => {
                self.prune(&mut hits, now);
                hits.len() < self.limit
            }
            Err(_) => true,
        }
    }

    /// Seconds until the window has room again, or `None` if it has room now.
    pub fn retry_after_seconds(&self) -> Option<i64> {
        if self.is_disabled() {
            return None;
        }
        let now = Instant::now();
        let mut hits = self.hits.lock().ok()?;
        self.prune(&mut hits, now);
        if hits.len() < self.limit {
            return None;
        }
        hits.front().map(|oldest| {
            let elapsed = now.duration_since(*oldest);
            self.window.saturating_sub(elapsed).as_secs() as i64
        })
    }

    /// Current occupancy (for logging / diagnostics).
    pub fn current(&self) -> usize {
        let now = Instant::now();
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
            assert!(bucket.try_acquire(), "hit {i} should be admitted");
        }
        assert!(!bucket.try_acquire(), "4th hit must be refused");
        assert_eq!(bucket.current(), 3);
        assert!(bucket.retry_after_seconds().is_some());
    }

    /// The whole point: many distinct identities share ONE budget, so rotating
    /// between them gains nothing. (Identity-blindness is what makes this true;
    /// the end-to-end proof across real rotating IPs lives in the handler tests
    /// in `routes.rs`, since the bucket itself has no concept of an IP.)
    #[test]
    fn aggregate_bucket_is_not_per_identity() {
        let bucket = AggregateBucket::new(2, 60);
        assert!(bucket.try_acquire()); // exit A
        assert!(bucket.try_acquire()); // exit B
        assert!(
            !bucket.try_acquire(),
            "a third distinct exit must NOT get its own allowance"
        );
    }

    /// `try_acquire` must be the atomic admission authority: N threads racing
    /// must never admit more than `limit` in total.
    #[test]
    fn aggregate_bucket_try_acquire_is_atomic_under_contention() {
        use std::sync::Arc;
        const LIMIT: usize = 25;
        const THREADS: usize = 16;
        const PER_THREAD: usize = 20;

        let bucket = Arc::new(AggregateBucket::new(LIMIT, 60));
        let mut handles = Vec::new();
        for _ in 0..THREADS {
            let b = Arc::clone(&bucket);
            handles.push(std::thread::spawn(move || {
                (0..PER_THREAD).filter(|_| b.try_acquire()).count()
            }));
        }
        let admitted: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
        assert_eq!(
            admitted,
            LIMIT,
            "exactly {LIMIT} of {} racing attempts may be admitted",
            THREADS * PER_THREAD
        );
        assert_eq!(bucket.current(), LIMIT);
    }

    #[test]
    fn aggregate_bucket_release_returns_a_slot() {
        let bucket = AggregateBucket::new(1, 60);
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire(), "bucket is full");
        bucket.release();
        assert_eq!(bucket.current(), 0);
        assert!(bucket.try_acquire(), "released slot must be reusable");
    }

    /// A limit of 0 is the runtime off-switch: nothing is ever refused.
    #[test]
    fn aggregate_bucket_zero_limit_disables_the_ceiling() {
        let bucket = AggregateBucket::new(0, 60);
        assert!(bucket.is_disabled());
        for _ in 0..1000 {
            assert!(bucket.try_acquire(), "a disabled ceiling never refuses");
        }
        assert!(bucket.has_capacity());
        assert!(bucket.retry_after_seconds().is_none());
    }

    /// Sizing pin. Bounds are tied to the MEASURED organic peak, so raising the
    /// ceiling far above real traffic (or dropping it below it) fails here.
    ///
    /// Accepted collateral, deliberately NOT claimed away by this test: while
    /// an attacker holds the ceiling, ordinary Tor users are refused too. See
    /// `AggregateBucket` docs and freenet/web#81.
    #[test]
    #[allow(clippy::assertions_on_constants)] // deliberate: this pins the constants
    fn tor_ceiling_is_sized_between_organic_peak_and_burst() {
        // Measured 2026-07-24/25 against the official bulk exit list.
        const OBSERVED_ORGANIC_PEAK: usize = 33;
        const OBSERVED_BURST_HOUR: usize = 208;

        assert!(
            DEFAULT_TOR_INVITES_PER_HOUR > OBSERVED_ORGANIC_PEAK,
            "ceiling {DEFAULT_TOR_INVITES_PER_HOUR} must exceed the measured organic \
             peak {OBSERVED_ORGANIC_PEAK}/h or it refuses real users"
        );
        assert!(
            DEFAULT_TOR_INVITES_PER_HOUR <= 2 * OBSERVED_ORGANIC_PEAK,
            "ceiling {DEFAULT_TOR_INVITES_PER_HOUR} must stay within 2x the organic \
             peak {OBSERVED_ORGANIC_PEAK}/h or it barely constrains a burst"
        );
        assert!(
            DEFAULT_TOR_INVITES_PER_HOUR < OBSERVED_BURST_HOUR / 2,
            "ceiling must be well under the observed burst {OBSERVED_BURST_HOUR}/h"
        );
    }

    #[test]
    fn aggregate_bucket_expires_old_hits_individually() {
        let bucket = AggregateBucket::new(2, 60);
        {
            let mut hits = bucket.hits.lock().unwrap();
            // One aged out, one still inside the window.
            hits.push_back(Instant::now() - StdDuration::from_secs(61 * 60));
            hits.push_back(Instant::now() - StdDuration::from_secs(30 * 60));
        }
        assert_eq!(bucket.current(), 1, "only the expired hit is pruned");
        assert!(bucket.try_acquire(), "the freed slot is reusable");
        assert!(!bucket.try_acquire(), "and only one slot freed");
    }

    /// `retry_after_seconds` must reflect the OLDEST hit's expiry, not the
    /// newest. Reading `back()` instead of `front()` passes a naive test.
    #[test]
    fn retry_after_reflects_oldest_hit() {
        let bucket = AggregateBucket::new(2, 60);
        {
            let mut hits = bucket.hits.lock().unwrap();
            hits.push_back(Instant::now() - StdDuration::from_secs(59 * 60)); // frees in ~60s
            hits.push_back(Instant::now() - StdDuration::from_secs(60)); // frees in ~59min
        }
        let retry = bucket.retry_after_seconds().expect("bucket is full");
        assert!(
            (30..=120).contains(&retry),
            "expected ~60s from the OLDEST hit, got {retry}s (reading back() not front()?)"
        );
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
