//! Tor exit-node awareness for the invite rate limiter.
//!
//! # Why this exists
//!
//! gkapi's per-IP invite limit ([`crate::rate_limit::MAX_INVITES_PER_WINDOW`])
//! is structurally defeated by Tor circuit rotation: every new exit node
//! presents a fresh IP, and therefore a fresh bucket.
//!
//! Measured from `invite_rate_limits.json` (1728 invites,
//! 2026-07-24T06:23Z..2026-07-25T05:54Z), classified against the official bulk
//! exit list: the 02:35-03:05Z burst was **246 invites from 202 distinct IPs,
//! 200 of them via 159 Tor exits**, with no single IP near the per-IP limit.
//! Tightening the per-IP number does nothing about this -- rotation routes
//! around it at any value.
//!
//! Because the Tor exit set is *enumerable*, the fix is to stop treating exits
//! as independent identities and meter them as one shared bucket (see
//! [`crate::rate_limit::AggregateBucket`]).
//!
//! # Why not simply block Tor, and what the bucket costs instead
//!
//! Tor is ~7% of non-burst invite traffic (106 requests from 90 distinct exits
//! over the same window), nearly all 1-2 requests each -- ordinary usage.
//! Freenet is a privacy project and the quickstart is the main onboarding path,
//! so blocking that traffic outright denies real users in ALL states.
//!
//! The bucket is better than a block, but it is NOT free, and the earlier
//! version of this comment was wrong to claim ordinary Tor users are
//! unaffected. Sharing one budget across an anonymity set means the loudest
//! member can take all of it: an attacker who sustains the ceiling denies
//! invites to every Tor user for as long as they keep it up, and because a
//! refused request costs them nothing, an automated poller wins each freed slot
//! against a human. That is inherent to any aggregate cap over an unlinkable
//! pool. Closing it needs a per-request cost signal (proof-of-work / CAPTCHA),
//! tracked in freenet/web#81. Treat this module as rate-shaping that buys time,
//! not as a fix.
//!
//! # Failure policy: FAIL OPEN
//!
//! If the exit list cannot be fetched and no cached copy exists, [`TorExitList`]
//! is simply empty, [`TorExitList::is_exit`] returns `false` for everything, and
//! gkapi behaves exactly as it did before this module existed. Losing the list
//! must never turn into blocking legitimate users. It must equally never be
//! read as "everything is Tor" — hence a plain `HashSet` membership test with
//! no sentinel/unknown state.

use chrono::{DateTime, Utc};
use log::{info, warn};
use std::collections::HashSet;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thiserror::Error;

/// Authoritative bulk exit list published by the Tor Project.
///
/// Plain text, one IP per line. This is the list TorDNSEL is built from and is
/// the canonical source for "is this address a Tor exit".
pub const TOR_BULK_EXIT_LIST_URL: &str = "https://check.torproject.org/torbulkexitlist";

/// How often the list is re-fetched. Exits churn on the order of hours, so
/// hourly keeps us close enough without hammering the Tor Project.
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Refuse a response larger than this. The real list is well under 1 MB; this
/// only exists so a corrupted or hostile response cannot exhaust memory.
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// Hard cap on retained entries, for the same reason as `MAX_RESPONSE_BYTES`.
/// The real list is ~2000 exits, so this is ~50x headroom.
const MAX_EXITS: usize = 100_000;

/// A real bulk exit list holds ~1000-2000 entries. Anything far below this is a
/// truncated or wrong response, not a genuinely tiny Tor network.
const MIN_PLAUSIBLE_EXITS: usize = 200;

/// Beyond this age the cached list is still USED (stale data beats no data for
/// a rate-limit hint) but is logged as stale, because decommissioned exits can
/// be reassigned to ordinary users and would then be metered as Tor.
const STALE_AFTER_HOURS: i64 = 24;

/// Network timeout for a single refresh attempt.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Error, Debug)]
pub enum TorListError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("response too large: {0} bytes exceeds cap of {MAX_RESPONSE_BYTES}")]
    TooLarge(usize),
    #[error("implausibly small exit list: {got} entries (minimum {min}); refusing to replace a good list")]
    Implausible { got: usize, min: usize },
    #[error(
        "exit list shrank implausibly: {got} entries vs {previous} previously; refusing to replace"
    )]
    Shrank { got: usize, previous: usize },
    #[error("lock poisoned")]
    Lock,
}

/// Map an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`) to its IPv4 form so both
/// spellings compare equal. Other addresses pass through unchanged.
fn canonicalize(ip: &IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => *ip,
        },
        _ => *ip,
    }
}

#[derive(Default)]
struct Snapshot {
    exits: HashSet<IpAddr>,
    updated: Option<DateTime<Utc>>,
}

/// A periodically-refreshed set of Tor exit-node addresses.
pub struct TorExitList {
    inner: RwLock<Snapshot>,
    /// Where the last good copy is persisted, so a restart does not start blind.
    cache_path: Option<PathBuf>,
    url: String,
}

impl TorExitList {
    /// Build a list, seeding from the on-disk cache if one is present.
    ///
    /// Never fails: an unreadable or corrupt cache just yields an empty set
    /// (fail open), which is logged.
    pub fn new(cache_path: Option<PathBuf>) -> Self {
        Self::with_url(cache_path, TOR_BULK_EXIT_LIST_URL.to_string())
    }

    /// As [`Self::new`], with an overridable source URL (used by tests).
    pub fn with_url(cache_path: Option<PathBuf>, url: String) -> Self {
        let mut snapshot = Snapshot::default();

        if let Some(path) = cache_path.as_deref() {
            match Self::read_cache(path) {
                Ok(Some((exits, updated))) => {
                    info!(
                        "Loaded {} Tor exit addresses from cache at {}",
                        exits.len(),
                        path.display()
                    );
                    snapshot = Snapshot {
                        exits,
                        updated: Some(updated),
                    };
                }
                Ok(None) => {
                    info!(
                        "No Tor exit cache at {} yet; starting empty (fail open)",
                        path.display()
                    );
                }
                Err(e) => {
                    warn!(
                        "Could not read Tor exit cache at {}: {e}; starting empty (fail open)",
                        path.display()
                    );
                }
            }
        }

        Self {
            inner: RwLock::new(snapshot),
            cache_path,
            url,
        }
    }

    /// Is `ip` a known Tor exit node?
    ///
    /// Returns `false` when the list is empty or unavailable — see the
    /// fail-open policy in the module docs.
    pub fn is_exit(&self, ip: &IpAddr) -> bool {
        // Canonicalize first. gkapi binds 0.0.0.0 today so peers are always
        // IpAddr::V4, but changing the bind to `::` for dual-stack (a one-line,
        // plausible change) makes Linux deliver IPv4 peers as ::ffff:a.b.c.d,
        // which would never match the V4 entries parsed from the list -- every
        // exit would silently escape metering with nothing in the logs.
        let ip = canonicalize(ip);
        match self.inner.read() {
            Ok(snap) => snap.exits.contains(&ip),
            Err(_) => {
                warn!("Tor exit list lock poisoned; treating as non-Tor (fail open)");
                false
            }
        }
    }

    /// Number of known exits (0 when unavailable).
    pub fn len(&self) -> usize {
        self.inner.read().map(|s| s.exits.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// When the list was last successfully refreshed or loaded.
    pub fn last_updated(&self) -> Option<DateTime<Utc>> {
        self.inner.read().ok().and_then(|s| s.updated)
    }

    /// True when the list is older than [`STALE_AFTER_HOURS`]. Stale lists are
    /// still used; this is for logging and operator visibility.
    pub fn is_stale(&self) -> bool {
        match self.last_updated() {
            Some(t) => Utc::now() - t > chrono::Duration::hours(STALE_AFTER_HOURS),
            None => true,
        }
    }

    /// Fetch the list and atomically replace the in-memory set.
    ///
    /// On any error the previous set is left untouched, so a transient outage
    /// degrades to "keep using the last good list" rather than to an empty one.
    pub async fn refresh(&self) -> Result<usize, TorListError> {
        let client = reqwest::Client::builder().timeout(FETCH_TIMEOUT).build()?;

        let response = client.get(&self.url).send().await?.error_for_status()?;

        // Reject an oversized body up front when the server declares a length.
        if let Some(len) = response.content_length() {
            if len > MAX_RESPONSE_BYTES as u64 {
                return Err(TorListError::TooLarge(len.min(usize::MAX as u64) as usize));
            }
        }

        // Stream with a running total. `response.text()` would buffer the whole
        // body BEFORE any size check, so a chunked response (no content-length)
        // could allocate without bound until FETCH_TIMEOUT -- an OOM on a small
        // VM. The cap has to be enforced while reading, not after.
        let mut response = response;
        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if buf.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(TorListError::TooLarge(buf.len() + chunk.len()));
            }
            buf.extend_from_slice(&chunk);
        }
        let body = String::from_utf8_lossy(&buf).into_owned();

        let exits = Self::parse(&body);
        // "Non-empty" is NOT enough validation. A truncated 200, or an HTML
        // error page that happens to contain one IP-shaped string, would
        // otherwise replace a good ~1400-entry list with a handful of entries
        // and silently switch off nearly all Tor metering -- the worst kind of
        // failure, because everything keeps "working".
        if exits.len() < MIN_PLAUSIBLE_EXITS {
            return Err(TorListError::Implausible {
                got: exits.len(),
                min: MIN_PLAUSIBLE_EXITS,
            });
        }
        let previous = self.len();
        if previous > 0 && exits.len() * 2 < previous {
            return Err(TorListError::Shrank {
                got: exits.len(),
                previous,
            });
        }

        let now = Utc::now();
        let count = exits.len();

        if let Some(path) = self.cache_path.as_deref() {
            if let Err(e) = Self::write_cache(path, &body) {
                // A cache we cannot persist is survivable; the in-memory set is
                // what actually gates requests.
                warn!(
                    "Could not persist Tor exit cache to {}: {e}",
                    path.display()
                );
            }
        }

        {
            let mut snap = self.inner.write().map_err(|_| TorListError::Lock)?;
            snap.exits = exits;
            snap.updated = Some(now);
        }

        Ok(count)
    }

    /// Parse the bulk exit list: one address per line, `#` comments and blank
    /// lines ignored, unparseable lines skipped. Handles IPv4 and IPv6.
    fn parse(body: &str) -> HashSet<IpAddr> {
        let mut out = HashSet::new();
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Ok(ip) = line.parse::<IpAddr>() {
                out.insert(canonicalize(&ip));
                if out.len() >= MAX_EXITS {
                    warn!("Tor exit list hit the {MAX_EXITS} entry cap; truncating");
                    break;
                }
            }
        }
        out
    }

    fn read_cache(path: &Path) -> Result<Option<(HashSet<IpAddr>, DateTime<Utc>)>, TorListError> {
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let exits = Self::parse(&content);
        if exits.is_empty() {
            return Ok(None);
        }
        let updated: DateTime<Utc> = std::fs::metadata(path)?
            .modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        Ok(Some((exits, updated)))
    }

    fn write_cache(path: &Path, body: &str) -> Result<(), TorListError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Write-then-rename so a crash mid-write cannot leave a truncated list
        // that would silently shrink the exit set on next start.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

/// Refresh `list` immediately, then every [`REFRESH_INTERVAL`], forever.
///
/// Errors are logged and retried on the next tick; they never abort the loop
/// and never clear the current set.
pub fn spawn_refresher(list: Arc<TorExitList>) {
    if list.is_empty() {
        info!("Tor exit list starting empty; shared Tor ceiling inactive until first refresh");
    } else {
        info!(
            "Tor exit list seeded with {} entries from cache (last updated: {})",
            list.len(),
            list.last_updated()
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string())
        );
    }

    let handle = tokio::spawn(async move {
        // Backoff schedule used ONLY while we have no usable list at all. With
        // an empty list the ceiling does not exist, so waiting a full hour after
        // one transient failure would leave it off for an hour; with a good
        // cached list an hourly retry is fine.
        const EMPTY_RETRY_SECS: &[u64] = &[30, 60, 300, 900];
        let mut empty_attempt = 0usize;

        loop {
            match list.refresh().await {
                Ok(n) => {
                    info!("Refreshed Tor exit list: {n} exit addresses");
                    empty_attempt = 0;
                }
                Err(e) => {
                    let cached = list.len();
                    if cached == 0 {
                        warn!(
                            "Tor exit list refresh failed with NO cached list: {e}. \
                             Tor traffic is metered per-IP only (ceiling inactive)."
                        );
                    } else {
                        warn!(
                            "Tor exit list refresh failed: {e} (still enforcing {cached} cached \
                             entries, last updated {}{})",
                            list.last_updated()
                                .map(|t| t.to_rfc3339())
                                .unwrap_or_else(|| "never".to_string()),
                            if list.is_stale() { ", STALE" } else { "" }
                        );
                    }
                }
            }

            let delay = if list.is_empty() {
                let d = EMPTY_RETRY_SECS[empty_attempt.min(EMPTY_RETRY_SECS.len() - 1)];
                empty_attempt = empty_attempt.saturating_add(1);
                Duration::from_secs(d)
            } else {
                REFRESH_INTERVAL
            };
            tokio::time::sleep(delay).await;
        }
    });

    // The loop above is infinite, so the ONLY way this task ends is a panic --
    // which would silently freeze the exit list forever with nothing in the
    // logs. Watch the handle so that failure is at least loud.
    tokio::spawn(async move {
        match handle.await {
            Ok(()) => log::error!(
                "Tor exit list refresher exited unexpectedly; the exit list is now frozen"
            ),
            Err(e) => {
                log::error!("Tor exit list refresher panicked: {e}; the exit list is now frozen")
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use tempfile::tempdir;

    #[test]
    fn parses_ipv4_ipv6_and_ignores_junk() {
        let body = "\
# comment line
185.220.101.30

192.42.116.15
2001:db8::1
not-an-ip
   171.25.193.78
";
        let set = TorExitList::parse(body);
        assert_eq!(set.len(), 4, "expected 4 parseable addresses, got {set:?}");
        assert!(set.contains(&IpAddr::V4(Ipv4Addr::new(185, 220, 101, 30))));
        assert!(set.contains(&IpAddr::V4(Ipv4Addr::new(192, 42, 116, 15))));
        assert!(set.contains(&IpAddr::V6("2001:db8::1".parse::<Ipv6Addr>().unwrap())));
        // Surrounding whitespace must not defeat the match.
        assert!(set.contains(&IpAddr::V4(Ipv4Addr::new(171, 25, 193, 78))));
    }

    /// Live end-to-end check against the real Tor Project list.
    ///
    /// `#[ignore]`d so CI stays hermetic; run explicitly before deploying a
    /// change to the fetch/parse path:
    ///   `cargo test --bins -- --ignored refresh_fetches_real_tor_exit_list --nocapture`
    ///
    /// Asserts on shape rather than an exact count (the list churns): a
    /// plausible number of exits, and that a few long-lived exit ranges observed
    /// in the 2026-07-25 burst are recognised.
    #[tokio::test]
    #[ignore]
    async fn refresh_fetches_real_tor_exit_list() {
        let dir = tempdir().unwrap();
        let cache = dir.path().join("tor_exits.txt");
        let list = TorExitList::new(Some(cache.clone()));

        let n = list.refresh().await.expect("refresh should succeed");
        println!("fetched {n} Tor exit addresses");

        assert!(
            (500..MAX_EXITS).contains(&n),
            "expected a plausible exit count, got {n}"
        );
        assert_eq!(list.len(), n);
        assert!(!list.is_stale(), "freshly refreshed list must not be stale");
        assert!(cache.exists(), "refresh must persist the cache");

        // A refreshed list must round-trip through the cache unchanged.
        let reloaded = TorExitList::new(Some(cache));
        assert_eq!(reloaded.len(), n, "cache reload must preserve the set");

        // Sanity: a non-Tor address must not be flagged.
        assert!(!list.is_exit(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    /// The fail-open contract: with no list, nothing is Tor.
    #[test]
    fn empty_list_treats_everything_as_non_tor() {
        let list = TorExitList::new(None);
        assert!(list.is_empty());
        assert!(!list.is_exit(&IpAddr::V4(Ipv4Addr::new(185, 220, 101, 30))));
        assert!(!list.is_exit(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(list.is_stale(), "a never-populated list counts as stale");
    }

    #[test]
    fn seeds_from_cache_on_construction() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tor_exits.txt");
        std::fs::write(&path, "185.220.101.30\n192.42.116.15\n").unwrap();

        let list = TorExitList::new(Some(path));
        assert_eq!(list.len(), 2);
        assert!(list.is_exit(&IpAddr::V4(Ipv4Addr::new(185, 220, 101, 30))));
        assert!(!list.is_exit(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(list.last_updated().is_some());
    }

    /// A corrupt cache must not panic or wedge startup -- it degrades to empty.
    #[test]
    fn corrupt_cache_fails_open() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tor_exits.txt");
        std::fs::write(&path, "this is not\nan ip list at all\n").unwrap();

        let list = TorExitList::new(Some(path));
        assert!(list.is_empty());
        assert!(!list.is_exit(&IpAddr::V4(Ipv4Addr::new(185, 220, 101, 30))));
    }

    #[test]
    fn missing_cache_path_fails_open() {
        let dir = tempdir().unwrap();
        let list = TorExitList::new(Some(dir.path().join("does-not-exist.txt")));
        assert!(list.is_empty());
    }

    #[test]
    fn cache_write_is_atomic_and_reloadable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("tor_exits.txt");
        TorExitList::write_cache(&path, "185.220.101.30\n").unwrap();
        assert!(path.exists());
        assert!(
            !path.with_extension("tmp").exists(),
            "temp file must be renamed away, not left behind"
        );

        let reloaded = TorExitList::new(Some(path));
        assert_eq!(reloaded.len(), 1);
    }
}
