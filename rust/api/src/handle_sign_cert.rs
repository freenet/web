use std::collections::HashMap;
use std::str::FromStr;

use blind_rsa_signatures::BlindedMessage;
use serde::{Deserialize, Serialize};
use stripe::{Client, PaymentIntent, PaymentIntentStatus};

use ghostkey_lib::armorable::Armorable;

use crate::delegates::sign_with_notary_key;
pub use crate::errors::CertificateError;

#[derive(Debug, Deserialize)]
pub struct SignCertificateRequest {
    payment_intent_id: String,
    blinded_ghost_key_base64: String,
}

/// HTTP response for successful certificate signing.
///
/// During the 0.2.0 rename transition the notary certificate is emitted in
/// BOTH `delegate_certificate_base64` (legacy) and `notary_certificate_base64`
/// (canonical) fields with identical values. This lets already-cached browser
/// JS (which only reads the legacy name) keep working while freshly served
/// JS picks up the new field. The legacy field is slated for removal in a
/// future release. See freenet/web#24.
#[derive(Debug, Serialize)]
pub struct SignCertificateResponse {
    pub blind_signature_base64: String,
    pub delegate_certificate_base64: String,
    pub notary_certificate_base64: String,
    pub amount: u64,
}

pub async fn sign_certificate(
    request: SignCertificateRequest,
) -> Result<SignCertificateResponse, CertificateError> {
    log::info!(
        "Starting sign_certificate function with request: {:?}",
        request
    );
    log::debug!("Current working directory: {:?}", std::env::current_dir());
    log::debug!("HOME environment variable: {:?}", std::env::var("HOME"));

    let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(|e| {
        log::error!("Environment variable STRIPE_SECRET_KEY not found: {}", e);
        log::error!(
            "Current environment variables: {:?}",
            std::env::vars().collect::<Vec<_>>()
        );
        CertificateError::KeyError("STRIPE_SECRET_KEY environment variable not set".to_string())
    })?;

    log::info!("STRIPE_SECRET_KEY found");
    let client = Client::new(stripe_secret_key);

    // Take an exclusive claim on this PaymentIntent and hold it for the rest of
    // the function. The `certificate_signed` check below and the update that
    // sets it are two separate Stripe calls with nothing atomic between them,
    // so without this, concurrent requests carrying the same PaymentIntent all
    // observe an unset flag and all go on to sign, minting several Ghost Keys
    // from one donation. See the payment_claim module for why that specific
    // failure matters more than an ordinary double-submit.
    let _claim = crate::payment_claim::claim(&request.payment_intent_id).await;

    // Verify payment intent
    let pi = PaymentIntent::retrieve(
        &client,
        &stripe::PaymentIntentId::from_str(&request.payment_intent_id)?,
        &[],
    )
    .await
    .map_err(|e| {
        log::error!("Failed to retrieve PaymentIntent: {:?}", e);
        CertificateError::StripeError(e)
    })?;

    log::info!("Retrieved PaymentIntent: {:?}", pi);
    log::info!("PaymentIntent status: {:?}", pi.status);

    match pi.status {
        PaymentIntentStatus::Succeeded => {
            // Proceed with certificate signing
        }
        PaymentIntentStatus::RequiresPaymentMethod => {
            log::error!("Payment method is missing. Status: {:?}", pi.status);
            return Err(CertificateError::PaymentMethodMissing);
        }
        _ => {
            log::error!("Payment not successful. Status: {:?}", pi.status);
            return Err(CertificateError::PaymentNotSuccessful);
        }
    }

    // Check if the certificate has already been signed
    if pi.metadata.get("certificate_signed").is_some() {
        log::warn!("Certificate already signed for PaymentIntent: {}", pi.id);
        return Err(CertificateError::CertificateAlreadySigned);
    }

    // Parse the caller-supplied key BEFORE marking the PaymentIntent as spent.
    // A malformed request is the caller's mistake and must not consume the
    // donation; marking first would leave a donor charged with nothing to show
    // for it and no way to retry.
    let blinded_ghostkey =
        BlindedMessage::from_base64(&request.blinded_ghost_key_base64).map_err(|e| {
            log::error!("Error in from_base64: {:?}", e);
            CertificateError::MiscError(e.to_string())
        })?;

    // Mark the payment intent as used for certificate signing
    let mut metadata = HashMap::new();
    metadata.insert("certificate_signed".to_string(), "true".to_string());
    let params = stripe::UpdatePaymentIntent {
        metadata: Some(metadata),
        ..Default::default()
    };
    PaymentIntent::update(&client, &pi.id, params).await?;

    // Sign the certificate
    log::info!("Payment intent verified successfully");

    let amount_cents = pi.amount as u64;
    let amount_dollars = amount_cents / 100;

    match sign_marked_payment(&blinded_ghostkey, amount_dollars, amount_cents) {
        Ok(response) => Ok(response),
        Err(e) => {
            // The PaymentIntent is marked spent but no certificate came out of
            // it, so without this the donor is charged and permanently locked
            // out of retrying. Releasing the mark is safe here specifically
            // because `_claim` is still held: no concurrent request can slip
            // into the window where the flag is briefly clear again.
            release_certificate_mark(&client, &pi.id).await;
            Err(e)
        }
    }
}

/// Produce the signed certificate for a PaymentIntent that has already been
/// marked as spent.
///
/// Split out so the caller can tell "signing failed" apart from the earlier
/// validation steps and undo the mark for exactly that case.
fn sign_marked_payment(
    blinded_ghostkey: &BlindedMessage,
    amount_dollars: u64,
    amount_cents: u64,
) -> Result<SignCertificateResponse, CertificateError> {
    let blind_signature = sign_with_notary_key(blinded_ghostkey, amount_dollars).map_err(|e| {
        log::error!("Error in sign_with_notary_key: {:?}", e);
        e
    })?;

    let (notary_certificate, _) = crate::delegates::get_notary(amount_dollars)?;

    let cert_base64 = notary_certificate
        .to_base64()
        .map_err(|e| CertificateError::MiscError(e.to_string()))?;

    Ok(SignCertificateResponse {
        blind_signature_base64: blind_signature
            .to_base64()
            .map_err(|e| CertificateError::MiscError(e.to_string()))?,
        // Dual-emit: legacy name for cached browser JS, canonical name for
        // freshly served JS. Remove the legacy field in a future release (#24).
        delegate_certificate_base64: cert_base64.clone(),
        notary_certificate_base64: cert_base64,
        amount: amount_cents,
    })
}

/// Clear `certificate_signed` after a failed signing attempt, so the donation
/// can be retried.
///
/// Stripe deletes a metadata key when it is set to an empty string. A failure
/// here is logged rather than propagated: the caller is already returning the
/// original signing error, which is the more useful one to surface, and the
/// donation is recoverable by hand from the log line.
async fn release_certificate_mark(client: &Client, pi_id: &stripe::PaymentIntentId) {
    let mut metadata = HashMap::new();
    metadata.insert("certificate_signed".to_string(), String::new());
    let params = stripe::UpdatePaymentIntent {
        metadata: Some(metadata),
        ..Default::default()
    };

    if let Err(e) = PaymentIntent::update(client, pi_id, params).await {
        log::error!(
            "Signing failed for PaymentIntent {} AND clearing certificate_signed \
             failed: {:?}. This donation is now marked spent with no certificate \
             issued and needs to be cleared by hand before the donor can retry.",
            pi_id,
            e
        );
    } else {
        log::warn!(
            "Signing failed for PaymentIntent {}; cleared certificate_signed so \
             the donor can retry.",
            pi_id
        );
    }
}

#[cfg(test)]
mod tests {
    /// Strip all whitespace so the pins below survive rustfmt re-wrapping the
    /// lines they match.
    fn squeeze(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// Production source only. Without this cut the needles match their own
    /// text in this test module and every pin passes vacuously.
    fn production_source() -> String {
        let source = include_str!("handle_sign_cert.rs");
        let production = source
            .split_once("\nmod tests {")
            .map(|(before, _)| before)
            .expect("test module marker not found; the cut below is not working");
        squeeze(production)
    }

    /// The claim has to be taken before the flag is read, not after. Taking it
    /// afterwards leaves exactly the read-check-write window it exists to
    /// close, and nothing else in the test suite would notice: the happy path
    /// still returns a valid certificate.
    #[test]
    fn claim_is_taken_before_the_signed_flag_is_read() {
        let source = production_source();

        let claim_at = source
            .find(&squeeze("payment_claim::claim(&request.payment_intent_id)"))
            .expect("sign_certificate no longer claims the PaymentIntent at all");
        let check_at = source
            .find(&squeeze(r#"pi.metadata.get("certificate_signed")"#))
            .expect("the certificate_signed check has moved or been renamed");

        assert!(
            claim_at < check_at,
            "the PaymentIntent claim must be taken BEFORE certificate_signed is \
             read, otherwise concurrent requests can both observe an unset flag \
             and one donation mints several Ghost Keys"
        );
    }

    /// `let _ = claim(..)` drops the guard immediately and `let _claim = ..`
    /// holds it to end of scope. The two differ by one character and only the
    /// second one actually excludes anything, so pin the binding shape.
    #[test]
    fn claim_guard_is_bound_and_not_dropped_immediately() {
        let source = production_source();

        assert!(
            source.contains(&squeeze("let _claim = crate::payment_claim::claim(")),
            "the claim guard must be bound to a named binding that lives to the \
             end of sign_certificate"
        );
        assert!(
            !source.contains(&squeeze("let _ = crate::payment_claim::claim(")),
            "`let _ = claim(..)` drops the guard on the spot, so the claim is \
             released before the flag is even read and the race is fully open"
        );
    }

    /// A signing failure after the mark is set must clear it, or the donor is
    /// charged and permanently unable to retry.
    #[test]
    fn failed_signing_releases_the_mark() {
        let source = production_source();

        assert!(
            source.contains(&squeeze("release_certificate_mark(&client, &pi.id)")),
            "signing failures must clear certificate_signed, otherwise a \
             transient failure burns the donation"
        );
    }
}
