//! Source-address blocklist for invite issuance.
//!
//! Per-IP rate limiting bounds how fast one address can mint invites, but it
//! does nothing once an address is known to belong to an abuser: on 2026-07-26
//! a single address minted an invite, had the resulting member banned for hate
//! speech within two minutes, and came back 37 minutes later on the same
//! address to do it again. Both identities were banned; nothing stopped the
//! second one from being created.
//!
//! This module closes that loop. Every issued invite records which address
//! minted it, so that when the room moderator bans a member, the address behind
//! that member can be refused for a while.
//!
//! Two deliberate limits, so nobody mistakes this for more than it is:
//!
//! - It raises cost, it does not stop a determined actor. Someone who rotates
//!   addresses simply takes the next one. What it kills is the cheap case
//!   above, returning on the same address minutes after a ban.
//! - Addresses are not people. The address in that incident belonged to a
//!   commercial VPN provider, so a block can catch unrelated subscribers who
//!   share the exit. The block therefore covers invite issuance only, never
//!   Freenet or River themselves, and the refusal says how to proceed.

use chrono::{DateTime, Duration, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use thiserror::Error;

/// How long a source address stays blocked after one of its members is banned.
///
/// A week rather than a day. The addresses that reach this path are rented
/// hosting and VPN exits, where a longer block costs the abuser real money to
/// route around, and where the realistic collateral is a subscriber who wanted
/// an invite that week and can still get one from a friend or without the VPN.
pub const BLOCK_DURATION_DAYS: i64 = 7;

/// How long the member-to-source mapping is kept.
///
/// Must comfortably exceed `BLOCK_DURATION_DAYS`: a member can be banned days
/// after joining, and the mapping is what makes that ban actionable. Bounded so
/// the file cannot grow without limit.
pub const SOURCE_RETENTION_DAYS: i64 = 30;

/// An active block: the address, when it lifts, and the member whose ban set it.
pub type ActiveBlock = (IpAddr, DateTime<Utc>, String);

/// Upper bound on retained mappings, enforced oldest-first.
const MAX_TRACKED_SOURCES: usize = 100_000;

#[derive(Error, Debug)]
pub enum BlocklistError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Lock error")]
    Lock,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SourceRecord {
    ip: IpAddr,
    issued_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BlockRecord {
    blocked_until: DateTime<Utc>,
    /// The member whose ban caused this block, for operator review.
    member_id: String,
    blocked_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct BlocklistData {
    /// member_id -> address that minted that member's invite.
    sources: HashMap<String, SourceRecord>,
    /// address -> active block.
    blocks: HashMap<String, BlockRecord>,
}

/// Outcome of reporting a banned member, so callers can log precisely rather
/// than guessing why nothing happened.
#[derive(Debug, PartialEq, Eq)]
pub enum BanReport {
    /// The member's source address is now blocked until the given time.
    Blocked { ip: IpAddr, until: DateTime<Utc> },
    /// The block was already active; its expiry was extended.
    Extended { ip: IpAddr, until: DateTime<Utc> },
    /// No mapping for this member. Expected for members who joined before this
    /// feature shipped, or by an invite this service did not mint.
    UnknownMember,
}

pub struct InviteBlocklist {
    path: PathBuf,
    data: Mutex<BlocklistData>,
}

impl InviteBlocklist {
    /// Take the lock, recovering it if a previous holder panicked.
    ///
    /// Poisoning is sticky, so treating it as a hard failure would disable the
    /// blocklist for the rest of the process lifetime after a single panic,
    /// with only a log line as evidence. Recovering the guard keeps the control
    /// enforcing instead of silently switching it off, and still cannot take
    /// invite issuance down. The data behind it is a plain map that is rewritten
    /// whole on every mutation, so a panic cannot leave it half-updated in a way
    /// that matters.
    fn guard(&self) -> MutexGuard<'_, BlocklistData> {
        self.data.lock().unwrap_or_else(|poisoned| {
            warn!("Invite blocklist lock was poisoned; recovering and continuing to enforce");
            poisoned.into_inner()
        })
    }
    pub fn new(path: PathBuf) -> Self {
        let data = match Self::load(&path) {
            Ok(data) => data,
            Err(error) => {
                warn!("Could not load invite blocklist from {path:?}: {error}. Starting empty.");
                BlocklistData::default()
            }
        };
        info!(
            "Invite blocklist loaded: {} tracked source(s), {} active block(s)",
            data.sources.len(),
            data.blocks.len()
        );
        Self {
            path,
            data: Mutex::new(data),
        }
    }

    fn load(path: &PathBuf) -> Result<BlocklistData, BlocklistError> {
        if !path.exists() {
            return Ok(BlocklistData::default());
        }
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    }

    fn persist(path: &PathBuf, data: &BlocklistData) -> Result<(), BlocklistError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Write-then-rename so a crash mid-write cannot truncate the list into
        // an empty file, which would silently unblock every address.
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_string(data)?)?;
        fs::rename(&temporary, path)?;
        Ok(())
    }

    /// Drop expired blocks and mappings that have aged out. Also enforces the
    /// size ceiling, oldest first.
    fn prune(data: &mut BlocklistData, now: DateTime<Utc>) {
        data.blocks.retain(|_, block| block.blocked_until > now);
        let cutoff = now - Duration::days(SOURCE_RETENTION_DAYS);
        data.sources.retain(|_, source| source.issued_at > cutoff);
        if data.sources.len() > MAX_TRACKED_SOURCES {
            let mut by_age: Vec<(String, DateTime<Utc>)> = data
                .sources
                .iter()
                .map(|(member, source)| (member.clone(), source.issued_at))
                .collect();
            by_age.sort_by_key(|(_, issued_at)| *issued_at);
            let excess = data.sources.len() - MAX_TRACKED_SOURCES;
            for (member, _) in by_age.into_iter().take(excess) {
                data.sources.remove(&member);
            }
        }
    }

    /// Record which address minted an invite. Called after issuance succeeds.
    pub fn record_source(&self, member_id: &str, ip: IpAddr) -> Result<(), BlocklistError> {
        let now = Utc::now();
        let mut data = self.guard();
        data.sources
            .insert(member_id.to_string(), SourceRecord { ip, issued_at: now });
        Self::prune(&mut data, now);
        Self::persist(&self.path, &data)
    }

    /// Whether invite issuance from this address is currently refused.
    pub fn is_blocked(&self, ip: IpAddr) -> bool {
        let now = Utc::now();
        let data = self.guard();
        data.blocks
            .get(&ip.to_string())
            .is_some_and(|block| block.blocked_until > now)
    }

    /// Block the address behind a banned member.
    pub fn report_ban(&self, member_id: &str) -> Result<BanReport, BlocklistError> {
        let now = Utc::now();
        let until = now + Duration::days(BLOCK_DURATION_DAYS);
        let mut data = self.guard();
        let Some(source) = data.sources.get(member_id).cloned() else {
            return Ok(BanReport::UnknownMember);
        };
        let key = source.ip.to_string();
        let already_active = data
            .blocks
            .get(&key)
            .is_some_and(|block| block.blocked_until > now);
        data.blocks.insert(
            key,
            BlockRecord {
                blocked_until: until,
                member_id: member_id.to_string(),
                blocked_at: now,
            },
        );
        Self::prune(&mut data, now);
        Self::persist(&self.path, &data)?;
        Ok(if already_active {
            BanReport::Extended {
                ip: source.ip,
                until,
            }
        } else {
            BanReport::Blocked {
                ip: source.ip,
                until,
            }
        })
    }

    /// Member ids with a recorded source address.
    #[cfg(test)]
    pub fn recorded_members(&self) -> Vec<String> {
        let data = self.guard();
        let mut members: Vec<String> = data.sources.keys().cloned().collect();
        members.sort();
        members
    }

    /// Block an address directly, without going through a member ban.
    ///
    /// For the case where an operator already knows an address is hostile,
    /// including addresses whose invites predate the source ledger.
    pub fn block_ip(&self, ip: IpAddr, reason: &str) -> Result<DateTime<Utc>, BlocklistError> {
        let now = Utc::now();
        let until = now + Duration::days(BLOCK_DURATION_DAYS);
        let mut data = self.guard();
        data.blocks.insert(
            ip.to_string(),
            BlockRecord {
                blocked_until: until,
                member_id: reason.to_string(),
                blocked_at: now,
            },
        );
        Self::prune(&mut data, now);
        Self::persist(&self.path, &data)?;
        Ok(until)
    }

    /// Active blocks, for operator inspection.
    pub fn active_blocks(&self) -> Vec<ActiveBlock> {
        let now = Utc::now();
        let data = self.guard();
        let mut blocks: Vec<ActiveBlock> = data
            .blocks
            .iter()
            .filter(|(_, block)| block.blocked_until > now)
            .filter_map(|(ip, block)| {
                ip.parse::<IpAddr>()
                    .ok()
                    .map(|ip| (ip, block.blocked_until, block.member_id.clone()))
            })
            .collect();
        blocks.sort_by_key(|(_, until, _)| *until);
        blocks
    }

    /// Lift a block early. For an operator who decides a block caught the wrong
    /// people, which matters because these addresses can be shared VPN exits.
    pub fn unblock(&self, ip: IpAddr) -> Result<bool, BlocklistError> {
        let mut data = self.guard();
        let removed = data.blocks.remove(&ip.to_string()).is_some();
        if removed {
            Self::persist(&self.path, &data)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn blocklist(dir: &TempDir) -> InviteBlocklist {
        InviteBlocklist::new(dir.path().join("blocklist.json"))
    }

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    /// The 2026-07-26 sequence, which is the whole reason this module exists:
    /// one address mints an invite, that member is banned, the same address
    /// comes back for another.
    #[test]
    fn blocks_the_source_address_after_its_member_is_banned() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        let source = ip("170.62.100.54");

        list.record_source("S5VJWFCV", source).unwrap();
        assert!(!list.is_blocked(source), "clean address must be allowed");

        match list.report_ban("S5VJWFCV").unwrap() {
            BanReport::Blocked { ip, .. } => assert_eq!(ip, source),
            other => panic!("expected a fresh block, got {other:?}"),
        }
        assert!(list.is_blocked(source));
    }

    #[test]
    fn does_not_block_unrelated_addresses() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        list.record_source("BANNED", ip("170.62.100.54")).unwrap();
        list.record_source("INNOCENT", ip("73.11.36.49")).unwrap();
        list.report_ban("BANNED").unwrap();
        assert!(list.is_blocked(ip("170.62.100.54")));
        assert!(!list.is_blocked(ip("73.11.36.49")));
    }

    /// Only the exact address is blocked. Three other addresses in the same /24
    /// took invites that day and none of them produced a banned member, so
    /// widening to the subnet would have refused people on no evidence.
    #[test]
    fn blocks_a_single_address_not_its_neighbours() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        list.record_source("BANNED", ip("170.62.100.54")).unwrap();
        list.report_ban("BANNED").unwrap();
        for neighbour in ["170.62.100.43", "170.62.100.183", "170.62.100.204"] {
            assert!(
                !list.is_blocked(ip(neighbour)),
                "{neighbour} must be allowed"
            );
        }
    }

    #[test]
    fn reports_an_unknown_member_rather_than_blocking_nothing_silently() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        assert_eq!(
            list.report_ban("NEVER_SEEN").unwrap(),
            BanReport::UnknownMember
        );
        assert!(list.active_blocks().is_empty());
    }

    #[test]
    fn a_second_ban_from_the_same_address_extends_the_block() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        let source = ip("170.62.100.54");
        list.record_source("FIRST", source).unwrap();
        list.record_source("SECOND", source).unwrap();
        assert!(matches!(
            list.report_ban("FIRST").unwrap(),
            BanReport::Blocked { .. }
        ));
        assert!(matches!(
            list.report_ban("SECOND").unwrap(),
            BanReport::Extended { .. }
        ));
    }

    #[test]
    fn blocks_survive_restart() {
        let dir = TempDir::new().unwrap();
        let source = ip("170.62.100.54");
        {
            let list = blocklist(&dir);
            list.record_source("BANNED", source).unwrap();
            list.report_ban("BANNED").unwrap();
        }
        let reloaded = blocklist(&dir);
        assert!(
            reloaded.is_blocked(source),
            "a restart must not clear active blocks"
        );
    }

    /// The immediate operator case: block a known-hostile address without
    /// waiting for it to mint another invite.
    #[test]
    fn an_operator_can_block_an_address_directly() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        let source = ip("170.62.100.54");
        assert!(!list.is_blocked(source));
        list.block_ip(source, "manual: repeat hate-spam source")
            .unwrap();
        assert!(list.is_blocked(source));
        assert!(
            !list.is_blocked(ip("170.62.100.43")),
            "neighbour unaffected"
        );
    }

    #[test]
    fn an_operator_can_lift_a_block_early() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        let source = ip("170.62.100.54");
        list.record_source("BANNED", source).unwrap();
        list.report_ban("BANNED").unwrap();
        assert!(list.unblock(source).unwrap());
        assert!(!list.is_blocked(source));
        assert!(!list.unblock(source).unwrap(), "second lift is a no-op");
    }

    #[test]
    fn expired_blocks_stop_applying_and_are_pruned() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blocklist.json");
        let source = ip("170.62.100.54");
        let stale = Utc::now() - Duration::days(1);
        let mut data = BlocklistData::default();
        data.blocks.insert(
            source.to_string(),
            BlockRecord {
                blocked_until: stale,
                member_id: "OLD".into(),
                blocked_at: stale - Duration::days(BLOCK_DURATION_DAYS),
            },
        );
        InviteBlocklist::persist(&path, &data).unwrap();

        let list = InviteBlocklist::new(path);
        assert!(!list.is_blocked(source), "an expired block must not apply");
        assert!(list.active_blocks().is_empty());
    }

    #[test]
    fn a_block_lasts_a_week() {
        let dir = TempDir::new().unwrap();
        let list = blocklist(&dir);
        let source = ip("170.62.100.54");
        list.record_source("BANNED", source).unwrap();
        let BanReport::Blocked { until, .. } = list.report_ban("BANNED").unwrap() else {
            panic!("expected a fresh block");
        };
        let days = (until - Utc::now()).num_hours() as f64 / 24.0;
        assert!(
            (days - BLOCK_DURATION_DAYS as f64).abs() < 0.1,
            "expected a {BLOCK_DURATION_DAYS}-day block, got {days:.2} days"
        );
    }

    #[test]
    fn mappings_older_than_the_retention_window_are_dropped() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blocklist.json");
        let mut data = BlocklistData::default();
        data.sources.insert(
            "ANCIENT".into(),
            SourceRecord {
                ip: ip("170.62.100.54"),
                issued_at: Utc::now() - Duration::days(SOURCE_RETENTION_DAYS + 1),
            },
        );
        InviteBlocklist::persist(&path, &data).unwrap();

        let list = InviteBlocklist::new(path);
        // Pruning happens on the next write.
        list.record_source("RECENT", ip("73.11.36.49")).unwrap();
        assert_eq!(
            list.report_ban("ANCIENT").unwrap(),
            BanReport::UnknownMember
        );
    }
}
