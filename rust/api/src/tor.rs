//! Tor exit-node awareness for the invite rate limiter.
//!
//! # Why this exists
//!
//! gkapi's per-IP invite limit ([`crate::rate_limit::MAX_INVITES_PER_WINDOW`])
//! is structurally defeated by Tor circuit rotation: every new exit node
//! presents a fresh IP, and therefore a fresh bucket. Measured on 2026-07-25,
//! a single actor pulled **152 invites in ~20 minutes across 113 distinct exit
//! IPs**, and not one of those IPs came close to the per-IP limit. Tightening
//! the per-IP number does nothing about this — rotation simply routes around
//! it, whatever the number is.
//!
//! Because the Tor exit set is *enumerable*, the fix is to stop treating exits
//! as independent identities and meter them as one shared bucket (see
//! [`crate::rate_limit::AggregateBucket`]). Rotation then buys the attacker
//! nothing at all. This module supplies the membership test that makes that
//! possible.
//!
//! # Why not simply block Tor
//!
//! Measured over the same 23-hour window, Tor accounts for ~4.8% of invite
//! traffic *outside* the attack burst — 69 requests from 56 distinct exits,
//! nearly all of them 1-2 requests each, i.e. ordinary usage. Freenet is a
//! privacy project and the quickstart is the main onboarding path, so blocking
//! that traffic outright costs real users. A shared bucket sized above the
//! observed organic peak (11/hour) leaves those users untouched while capping
//! a rotation burst hard.
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
    #[error("response contained no parseable exit addresses")]
    Empty,
    #[error("lock poisoned")]
    Lock,
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
        match self.inner.read() {
            Ok(snap) => snap.exits.contains(ip),
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
            if len as usize > MAX_RESPONSE_BYTES {
                return Err(TorListError::TooLarge(len as usize));
            }
        }

        let body = response.text().await?;
        // ...and again after the fact, since content-length is advisory.
        if body.len() > MAX_RESPONSE_BYTES {
            return Err(TorListError::TooLarge(body.len()));
        }

        let exits = Self::parse(&body);
        if exits.is_empty() {
            // Never let a garbage 200 wipe a good list.
            return Err(TorListError::Empty);
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
                out.insert(ip);
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

    tokio::spawn(async move {
        loop {
            match list.refresh().await {
                Ok(n) => info!("Refreshed Tor exit list: {n} exit addresses"),
                Err(e) => warn!(
                    "Tor exit list refresh failed: {e} (using {} cached entries, last updated {}{}; \
                     Tor traffic is metered per-IP only until this succeeds)",
                    list.len(),
                    list.last_updated()
                        .map(|t| t.to_rfc3339())
                        .unwrap_or_else(|| "never".to_string()),
                    if list.is_stale() { ", STALE" } else { "" }
                ),
            }
            tokio::time::sleep(REFRESH_INTERVAL).await;
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
