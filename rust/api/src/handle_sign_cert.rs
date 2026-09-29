use std::collections::HashMap;
use std::str::FromStr;

use blind_rsa_signatures::BlindedMessage;
use serde::{Deserialize, Serialize};
use stripe::{Client, PaymentIntent, PaymentIntentStatus};

use ghostkey_lib::armorable::Armorable;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;

use crate::delegates::{quoted_notary, sign_with_notary_key, Notary, NOTARY_PERIOD_METADATA_KEY};
pub use crate::errors::CertificateError;

#[derive(Debug, Deserialize)]
pub struct SignCertificateRequest {
    payment_intent_id: String,
    blinded_ghost_key_base64: String,
    /// The notary certificate the client blinded against. Optional because
    /// browser JS cached from before this field existed does not send it.
    #[serde(default)]
    notary_certificate_base64: Option<String>,
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

    let amount_cents = pi.amount as u64;
    let amount_dollars = amount_cents / 100;

    // Sign with the pair this donation was quoted from (see
    // NOTARY_PERIOD_METADATA_KEY), not whichever is current now. Loaded before
    // marking the PaymentIntent spent, so a missing key cannot consume it.
    let notary = quoted_notary(
        amount_dollars,
        pi.metadata
            .get(NOTARY_PERIOD_METADATA_KEY)
            .map(String::as_str),
    )?;

    // If the client says which certificate it blinded against, refuse a
    // mismatch now, while the donation can still be retried. Signing anyway
    // would spend it on a signature that cannot unblind.
    if let Some(client_cert) = &request.notary_certificate_base64 {
        if !same_notary(client_cert, &notary)? {
            log::error!(
                "PaymentIntent {} was blinded against a different notary certificate \
                 than it was quoted; refusing before marking it spent",
                pi.id
            );
            return Err(CertificateError::NotaryMismatch);
        }
    }

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

    match sign_marked_payment(&blinded_ghostkey, &notary, amount_cents) {
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

/// Whether `client_cert` (base64, as the API handed it out) is `notary`'s
/// certificate. Compares the notary verifying key, which is what the client
/// blinds against.
fn same_notary(client_cert: &str, notary: &Notary) -> Result<bool, CertificateError> {
    let client_cert = NotaryCertificateV1::from_base64(client_cert)
        .map_err(|e| CertificateError::MiscError(format!("invalid notary certificate: {}", e)))?;
    let der = |k: &blind_rsa_signatures::PublicKey| {
        k.to_der()
            .map_err(|e| CertificateError::MiscError(e.to_string()))
    };
    Ok(der(&client_cert.payload.notary_verifying_key)?
        == der(&notary.certificate.payload.notary_verifying_key)?)
}

/// Produce the signed certificate for a PaymentIntent that has already been
/// marked as spent.
///
/// Split out so the caller can tell "signing failed" apart from the earlier
/// validation steps and undo the mark for exactly that case.
fn sign_marked_payment(
    blinded_ghostkey: &BlindedMessage,
    notary: &Notary,
    amount_cents: u64,
) -> Result<SignCertificateResponse, CertificateError> {
    let blind_signature =
        sign_with_notary_key(blinded_ghostkey, &notary.signing_key).map_err(|e| {
            log::error!("Error in sign_with_notary_key: {:?}", e);
            e
        })?;

    let cert_base64 = notary
        .certificate
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
    fn notary_is_loaded_and_checked_before_the_payment_is_marked_spent() {
        let source = production_source();

        let load_at = source
            .find(&squeeze("let notary = quoted_notary("))
            .expect("sign_certificate no longer loads the quoted notary");
        let check_at = source
            .find(&squeeze("return Err(CertificateError::NotaryMismatch);"))
            .expect("sign_certificate no longer refuses a notary mismatch");
        let mark_at = source
            .find(&squeeze(
                r#"metadata.insert("certificate_signed".to_string(), "true".to_string());"#,
            ))
            .expect("the certificate_signed mark has moved or been renamed");

        assert!(
            load_at < mark_at && check_at < mark_at,
            "a missing notary pair or a mismatched client certificate must fail \
             BEFORE certificate_signed is set, while the donor can still retry"
        );
    }

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

    fn notary(info: &str) -> super::Notary {
        let master = ed25519_dalek::SigningKey::generate(&mut rand_core::OsRng);
        let (certificate, signing_key) =
            super::NotaryCertificateV1::new(&master, &info.to_string()).unwrap();
        super::Notary {
            certificate,
            signing_key,
            period: None,
        }
    }

    #[test]
    fn same_notary_compares_the_key_the_client_blinded_against() {
        use super::Armorable;
        let quoted = notary("quoted");
        let other = notary("other");
        let quoted_b64 = quoted.certificate.to_base64().unwrap();
        let other_b64 = other.certificate.to_base64().unwrap();

        assert!(super::same_notary(&quoted_b64, &quoted).unwrap());
        assert!(!super::same_notary(&other_b64, &quoted).unwrap());
        assert!(super::same_notary("not a certificate", &quoted).is_err());
    }
}
