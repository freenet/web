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

/// One PaymentIntent's lock plus the number of requests that currently hold
/// or are waiting for it.
struct ClaimEntry {
    lock: Arc<AsyncMutex<()>>,
    /// Requests registered against this entry: the holder plus every queued
    /// waiter. Only ever read or written under the `CLAIM_LOCKS` lock.
    claimants: usize,
}

/// Live locks, keyed by PaymentIntent id.
///
/// Entries are removed when their last claimant goes away, whether it held
/// the lock or was cancelled while still waiting for it (see `Registration`),
/// so the map is bounded by the number of in-flight requests rather than by
/// the number of PaymentIntents ever seen. That bound is the point: the lock
/// is taken before the PaymentIntent is known to exist, so without cleanup an
/// unauthenticated caller could grow this map without limit by posting
/// garbage ids.
///
/// The claimant count is kept explicitly rather than inferred from
/// `Arc::strong_count` on the lock. tokio's `OwnedMutexGuard` releases the
/// mutex before it drops its own `Arc`, so in that gap a waiter on another
/// thread can acquire, finish and look at the count while the previous
/// holder's reference is still live. Each side then sees someone else still
/// using the entry, nobody removes it, and it leaks for good.
static CLAIM_LOCKS: LazyLock<Mutex<HashMap<String, ClaimEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Locks the claim map, recovering from poison.
///
/// A poisoned map lock only means some other thread panicked while holding
/// it. Every critical section below leaves the map structurally sound at each
/// step, and refusing to clean up would leak, so recover rather than
/// propagate.
fn claim_locks() -> std::sync::MutexGuard<'static, HashMap<String, ClaimEntry>> {
    CLAIM_LOCKS.lock().unwrap_or_else(|e| e.into_inner())
}

/// One request's registration as a claimant of a PaymentIntent's entry.
///
/// Created in `claim` in the same critical section that increments the
/// count, before anything is awaited, and withdrawn on drop. Because it is an
/// ordinary local until the lock is acquired, a request whose `claim` future
/// is dropped while still queued (axum drops the handler future when the
/// client disconnects) still withdraws, and so cannot leak the entry.
struct Registration {
    payment_intent_id: String,
}

impl Drop for Registration {
    fn drop(&mut self) {
        let mut map = claim_locks();
        let Some(entry) = map.get_mut(&self.payment_intent_id) else {
            // Unreachable: an entry is only removed when its count reaches
            // zero, and this registration is still counted. Logged as well as
            // asserted so a release build reports the broken invariant rather
            // than silently carrying on.
            log::error!(
                "payment claim entry missing for a live registration ({})",
                self.payment_intent_id
            );
            debug_assert!(false, "claim entry missing for a live registration");
            return;
        };
        entry.claimants -= 1;
        if entry.claimants == 0 {
            map.remove(&self.payment_intent_id);
        }
    }
}

/// Exclusive claim on one PaymentIntent, held for as long as the guard lives.
pub(crate) struct ClaimGuard {
    // Fields drop in declaration order, so the mutex is released before the
    // registration is withdrawn, and an entry only disappears once its mutex
    // is free. The other order would not break exclusion, since by the time a
    // guard is dropped its holder's critical section is over, but a newcomer
    // could then get a fresh mutex for the same PaymentIntent while the old
    // holder was still running its destructor. This order is the tidy one.
    _guard: OwnedMutexGuard<()>,
    _registration: Registration,
}

/// Wait until no other in-process request is signing against this
/// PaymentIntent, then take the claim.
pub(crate) async fn claim(payment_intent_id: &str) -> ClaimGuard {
    let payment_intent_id = payment_intent_id.to_string();
    let (lock, registration) = {
        let mut map = claim_locks();
        let entry = map
            .entry(payment_intent_id.clone())
            .or_insert_with(|| ClaimEntry {
                lock: Arc::new(AsyncMutex::new(())),
                claimants: 0,
            });
        entry.claimants += 1;
        (Arc::clone(&entry.lock), Registration { payment_intent_id })
    };

    // Awaited with the map lock released, so a slow claim on one PaymentIntent
    // never blocks claims on others. If this future is dropped here,
    // `registration` is dropped with it and the count comes back down.
    let guard = lock.lock_owned().await;

    ClaimGuard {
        _guard: guard,
        _registration: registration,
    }
}

/// Whether `id` is a well-formed PaymentIntent id: `pi_` followed by one or
/// more ASCII alphanumerics.
///
/// Callers check this before taking a claim or making any Stripe call, so
/// that only well-formed PaymentIntent ids are ever accepted.
pub(crate) fn is_payment_intent_id(id: &str) -> bool {
    id.strip_prefix("pi_")
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// Whether a specific PaymentIntent currently has a live entry.
///
/// Tests assert on individual keys rather than on the size of the map:
/// `CLAIM_LOCKS` is process-global and the test harness runs tests in parallel
/// threads, so any assertion about the total count is really an assertion about
/// what every other test in this module happens to be doing at that instant.
#[cfg(test)]
fn is_tracked(payment_intent_id: &str) -> bool {
    claim_locks().contains_key(payment_intent_id)
}

/// How many requests are registered against a PaymentIntent: its holder plus
/// every queued waiter, or 0 if it has no entry.
#[cfg(test)]
fn claimants(payment_intent_id: &str) -> usize {
    claim_locks()
        .get(payment_intent_id)
        .map_or(0, |entry| entry.claimants)
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn well_formed_payment_intent_ids_are_accepted() {
        for id in ["pi_1", "pi_3PqRsTuVwXyZ0123456789ab", "pi_ABCdef123"] {
            assert!(is_payment_intent_id(id), "{id:?} should be accepted");
        }
    }

    #[test]
    fn malformed_payment_intent_ids_are_rejected() {
        for id in [
            "",
            "pi_",
            "pi_abc-def",
            "pi_abc def",
            "pi_abc_def",
            "cus_123",
            "PI_123",
            "pi123",
        ] {
            assert!(!is_payment_intent_id(id), "{id:?} should be rejected");
        }
    }

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

        // Wait until the waiter has actually registered behind the held claim
        // rather than sleeping and hoping it has. The bound only exists to
        // turn a lost waiter into a failure instead of a hang.
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while claimants("pi_handoff") < 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the waiter never registered behind the held claim");
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

    /// axum drops a handler's future when the client disconnects, which can
    /// happen while the request is still queued behind another claim. The
    /// abandoned waiter never gets a guard, so it has to withdraw on its own
    /// or its key is never reclaimed.
    #[tokio::test]
    async fn cancelled_waiter_does_not_leak_its_key() {
        let held = claim("pi_cancelled_waiter").await;

        // The first poll registers the waiter and then parks it behind
        // `held`; the timeout then drops the still-queued future.
        let abandoned = tokio::time::timeout(
            std::time::Duration::from_millis(20),
            claim("pi_cancelled_waiter"),
        )
        .await;
        assert!(
            abandoned.is_err(),
            "the waiter acquired a claim that was still held"
        );
        assert!(
            is_tracked("pi_cancelled_waiter"),
            "entry vanished while its holder was still live"
        );

        drop(held);
        assert!(
            !is_tracked("pi_cancelled_waiter"),
            "a waiter cancelled while queued stranded its entry after the \
             holder released, so abandoned requests grow the map without bound"
        );

        // The key is still usable afterwards.
        drop(claim("pi_cancelled_waiter").await);
        assert!(!is_tracked("pi_cancelled_waiter"));
    }

    /// The same, for a waiter cancelled after the holder has already gone and
    /// before it was ever polled again: it must still be the one to clean up.
    #[tokio::test]
    async fn waiter_cancelled_after_holder_released_does_not_leak() {
        let held = claim("pi_cancelled_late").await;

        let mut waiter = Box::pin(claim("pi_cancelled_late"));
        // Poll once so it registers and queues, then release the holder
        // without ever polling the waiter again.
        let first_poll =
            std::future::poll_fn(|cx| std::task::Poll::Ready(waiter.as_mut().poll(cx))).await;
        assert!(first_poll.is_pending());
        drop(held);
        assert!(
            is_tracked("pi_cancelled_late"),
            "entry vanished while a waiter was still registered"
        );

        drop(waiter);
        assert!(
            !is_tracked("pi_cancelled_late"),
            "a waiter cancelled after the holder released stranded its entry"
        );
    }
}
