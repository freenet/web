//! Serializes certificate-signing attempts for a single PaymentIntent.
//!
//! The durable record that a PaymentIntent has already been spent on a Ghost
//! Key is its `certificate_signed` metadata flag in Stripe. Stripe offers no
//! compare-and-swap on metadata, so reading the flag, deciding, and then
//! setting it is three separate API calls with no atomicity between them. Two
//! requests carrying the same PaymentIntent could both observe an unset flag,
//! both set it, and both go on to sign, minting two Ghost Keys from one
//! donation.
//!
//! That matters more than an ordinary double-submit bug. Ghost Keys are sold
//! on the claim that an identity costs real money, so Sybil attacks get
//! expensive. An attacker who can mint N keys from one $1 donation by firing N
//! concurrent requests reduces that cost to nearly zero and the scarcity
//! property collapses.
//!
//! A per-PaymentIntent lock closes the window, which is the whole exposure
//! today: the API is a single axum process, so every request for a given
//! PaymentIntent contends on the same map. If it is ever run as more than one
//! instance behind a load balancer, this guard no longer spans them and the
//! claim has to move to shared storage. The Stripe flag remains the durable
//! record either way, so it still blocks a retry that arrives after the first
//! one finished, and still survives a restart.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

/// Live locks, keyed by PaymentIntent id.
///
/// Entries are removed when the last guard for a key is dropped (see
/// `ClaimGuard::drop`), so the map is bounded by the number of in-flight
/// requests rather than by the number of PaymentIntents ever seen. That
/// bound is the point: the lock is taken before the PaymentIntent is known to
/// exist, so without cleanup an unauthenticated caller could grow this map
/// without limit by posting garbage ids.
static CLAIM_LOCKS: LazyLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Exclusive claim on one PaymentIntent, held for as long as the guard lives.
pub(crate) struct ClaimGuard {
    payment_intent_id: String,
    // Dropped after `Drop::drop` runs, which is what makes the strong_count
    // arithmetic there work out.
    _guard: OwnedMutexGuard<()>,
}

impl Drop for ClaimGuard {
    fn drop(&mut self) {
        // A poisoned map lock only means some other thread panicked while
        // holding it; the map itself is still structurally sound, and refusing
        // to clean up would leak. Recover rather than propagate.
        let mut map = CLAIM_LOCKS.lock().unwrap_or_else(|e| e.into_inner());

        if let Some(lock) = map.get(&self.payment_intent_id) {
            // Two references means the map and this guard, so nobody else is
            // holding or waiting and the entry can go. Three or more means a
            // waiter has already cloned the Arc and must keep contending on
            // this same mutex, so leave it in place.
            if Arc::strong_count(lock) == 2 {
                map.remove(&self.payment_intent_id);
            }
        }
    }
}

/// Wait until no other in-process request is signing against this
/// PaymentIntent, then take the claim.
pub(crate) async fn claim(payment_intent_id: &str) -> ClaimGuard {
    let lock = {
        let mut map = CLAIM_LOCKS.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(payment_intent_id.to_string())
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone()
    };

    // Awaited with the map lock released, so a slow claim on one PaymentIntent
    // never blocks claims on others.
    let guard = lock.lock_owned().await;

    ClaimGuard {
        payment_intent_id: payment_intent_id.to_string(),
        _guard: guard,
    }
}

/// Whether a specific PaymentIntent currently has a live entry.
///
/// Tests assert on individual keys rather than on the size of the map:
/// `CLAIM_LOCKS` is process-global and the test harness runs tests in parallel
/// threads, so any assertion about the total count is really an assertion about
/// what every other test in this module happens to be doing at that instant.
#[cfg(test)]
fn is_tracked(payment_intent_id: &str) -> bool {
    CLAIM_LOCKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains_key(payment_intent_id)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// The property the whole module exists for: two concurrent claims on one
    /// PaymentIntent never overlap. Without the lock both tasks observe
    /// `inside == 0`, both proceed, and the peak is 2.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_claims_on_one_payment_intent_do_not_overlap() {
        let inside = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let mut tasks = Vec::new();
        for _ in 0..16 {
            let inside = Arc::clone(&inside);
            let peak = Arc::clone(&peak);
            tasks.push(tokio::spawn(async move {
                let _claim = claim("pi_contended").await;

                let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                // Long enough that overlapping tasks would reliably be caught
                // in the window together.
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                inside.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }

        assert_eq!(
            peak.load(Ordering::SeqCst),
            1,
            "two requests were inside the claim for one PaymentIntent at once, \
             so both could sign and one donation would mint two Ghost Keys"
        );
    }

    /// The lock must be per-PaymentIntent, not global, or one slow donation
    /// serializes everyone else's.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn claims_on_different_payment_intents_run_concurrently() {
        let inside = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let mut tasks = Vec::new();
        for i in 0..8 {
            let inside = Arc::clone(&inside);
            let peak = Arc::clone(&peak);
            tasks.push(tokio::spawn(async move {
                let _claim = claim(&format!("pi_distinct_{i}")).await;

                let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                inside.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        for t in tasks {
            t.await.unwrap();
        }

        assert!(
            peak.load(Ordering::SeqCst) > 1,
            "distinct PaymentIntents were serialized against each other"
        );
    }

    /// The lock is taken before the PaymentIntent is known to be real, so
    /// entries have to be reclaimed or unauthenticated garbage ids grow the
    /// map without bound.
    #[tokio::test]
    async fn released_claims_are_reclaimed() {
        let keys: Vec<String> = (0..64).map(|i| format!("pi_garbage_{i}")).collect();

        for key in &keys {
            let _claim = claim(key).await;
        }

        let leaked: Vec<&String> = keys.iter().filter(|k| is_tracked(k)).collect();
        assert!(
            leaked.is_empty(),
            "{} claim entries survived their guards, so a caller posting unknown \
             PaymentIntent ids can exhaust memory: {:?}",
            leaked.len(),
            leaked
        );
    }

    /// A waiter must keep contending on the same mutex the holder is using; if
    /// cleanup dropped the entry out from under it, the two would end up on
    /// different mutexes and the exclusion would silently stop working.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn entry_survives_while_a_waiter_is_queued() {
        let held = claim("pi_handoff").await;

        let waiter = tokio::spawn(async move {
            let _claim = claim("pi_handoff").await;
            // Still tracked while this second guard holds it.
            is_tracked("pi_handoff")
        });

        // Give the waiter time to queue behind the held claim.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            is_tracked("pi_handoff"),
            "entry vanished while a waiter was queued, so the waiter is now \
             contending on a different mutex than the holder and the exclusion \
             has silently stopped working"
        );

        drop(held);
        assert!(waiter.await.unwrap());
        assert!(
            !is_tracked("pi_handoff"),
            "entry outlived the last guard for this key"
        );
    }
}
