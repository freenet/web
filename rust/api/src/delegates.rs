//! Notary key lookup for the donation flow.
//!
//! This module was historically named "delegates" because the intermediate
//! PKI signing key was called a delegate. It was renamed to "notary" in 0.2.0
//! (issue freenet/web#24) to deconflict with Freenet's own `Delegate` (WASM
//! agent) concept. The module filename and the `DELEGATE_DIR` env var are
//! kept for backward compatibility with existing deployments; `NOTARY_DIR`
//! is the canonical name going forward.

use std::path::{Path, PathBuf};

use blind_rsa_signatures::{BlindSignature, BlindedMessage, Options, SecretKey as RSASigningKey};
use rand_core::OsRng;

use ghostkey_lib::armorable::*;
use ghostkey_lib::notary_certificate::NotaryCertificateV1;

use crate::handle_sign_cert::CertificateError;

/// Which naming scheme the per-amount files on disk use.
///
/// Resolved once per directory, not per file, so a partial migration on disk
/// cannot pair a new-named certificate with a legacy-named signing key. That
/// mismatch would produce ghost-key certificates whose signatures don't chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamingScheme {
    /// `notary_certificate_{amount}.pem` / `notary_signing_key_{amount}.pem`
    Notary,
    /// `delegate_certificate_{amount}.pem` / `delegate_signing_key_{amount}.pem`
    LegacyDelegate,
}

impl NamingScheme {
    fn cert_filename(&self, amount: u64) -> String {
        match self {
            Self::Notary => format!("notary_certificate_{}.pem", amount),
            Self::LegacyDelegate => format!("delegate_certificate_{}.pem", amount),
        }
    }

    fn signing_key_filename(&self, amount: u64) -> String {
        match self {
            Self::Notary => format!("notary_signing_key_{}.pem", amount),
            Self::LegacyDelegate => format!("delegate_signing_key_{}.pem", amount),
        }
    }
}

/// Resolve the notary directory from env, preferring `NOTARY_DIR` and
/// falling back to the legacy `DELEGATE_DIR` with a deprecation warning.
fn notary_dir() -> Result<PathBuf, CertificateError> {
    if let Ok(dir) = std::env::var("NOTARY_DIR") {
        return Ok(PathBuf::from(dir));
    }
    if let Ok(dir) = std::env::var("DELEGATE_DIR") {
        log::warn!(
            "reading legacy DELEGATE_DIR env var; rename to NOTARY_DIR — \
             DELEGATE_DIR will be removed in a future release (freenet/web#24)"
        );
        return Ok(PathBuf::from(dir));
    }
    log::error!("neither NOTARY_DIR nor DELEGATE_DIR is set");
    Err(CertificateError::KeyError(
        "NOTARY_DIR environment variable not set".to_string(),
    ))
}

/// Decide which naming scheme a directory uses for a given amount. Prefers
/// the canonical `notary_*` pair; falls back to the legacy `delegate_*` pair
/// only if BOTH legacy files exist AND neither canonical file does.
///
/// A partial migration where only the cert has been renamed (or only the
/// signing key) resolves to the canonical scheme and the reader surfaces a
/// "not found" error referencing the new name — that's safer than silently
/// pairing a new cert with a legacy signing key, which would issue ghost
/// certs whose signatures don't verify.
fn pick_scheme(dir: &Path, amount: u64) -> NamingScheme {
    let notary_cert = dir.join(NamingScheme::Notary.cert_filename(amount));
    let notary_key = dir.join(NamingScheme::Notary.signing_key_filename(amount));
    if notary_cert.exists() && notary_key.exists() {
        return NamingScheme::Notary;
    }

    let legacy_cert = dir.join(NamingScheme::LegacyDelegate.cert_filename(amount));
    let legacy_key = dir.join(NamingScheme::LegacyDelegate.signing_key_filename(amount));
    if legacy_cert.exists() && legacy_key.exists() {
        log::warn!(
            "reading legacy per-amount files delegate_certificate_{0}.pem / \
             delegate_signing_key_{0}.pem; rename to notary_*_{0}.pem — the \
             legacy filenames will be removed in a future release (freenet/web#24)",
            amount
        );
        return NamingScheme::LegacyDelegate;
    }

    // Neither complete pair exists. Default to canonical so the downstream
    // read surfaces a clean "not found" error referencing the new name.
    NamingScheme::Notary
}

/// PaymentIntent metadata key recording which notary period a donation was
/// quoted from: `YYYY-MM`, or absent for the flat per-amount files.
///
/// A donation's certificate is fixed when it is quoted (`/create-donation`,
/// `/update-donation`), but the ghost key is blinded and signed only after the
/// card is charged, possibly in another month. The success page blinds against
/// this quoted certificate (fetched from `/notary-certificate/{id}`), so
/// `/sign-certificate` must sign with the same pair, not whichever is current by
/// then, or the donor is charged for a signature that does not unblind.
pub(crate) const NOTARY_PERIOD_METADATA_KEY: &str = "notary_period";

/// A notary pair, and the period directory it came from (`None` for the flat
/// per-amount files directly under the notary directory).
pub(crate) struct Notary {
    pub certificate: NotaryCertificateV1,
    pub signing_key: RSASigningKey,
    pub period: Option<String>,
}

/// `YYYY-MM` with a real month. Also what keeps a period read back from
/// metadata from naming a path outside the notary directory.
fn is_period(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 7
        && b[4] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..].iter().all(u8::is_ascii_digit)
        && matches!(
            &name[5..],
            "01" | "02" | "03" | "04" | "05" | "06" | "07" | "08" | "09" | "10" | "11" | "12"
        )
}

/// The period to quote new donations from: the newest `YYYY-MM` directory that
/// is not in the future (generate_notary_keys.sh --start-month writes one per
/// month). `None` means there is no schedule yet, so use the flat files.
fn current_period(dir: &Path, now: chrono::DateTime<chrono::Utc>) -> Option<String> {
    let this_month = now.format("%Y-%m").to_string();
    let newest = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| is_period(name) && *name <= this_month)
        .max()?;
    if newest != this_month {
        log::error!(
            "notary schedule has no directory for {}; still issuing from {}. \
             Generate more months with generate_notary_keys.sh --start-month.",
            this_month,
            newest
        );
    }
    Some(newest)
}

/// The notary pair to quote a new donation from.
pub(crate) fn current_notary(amount: u64) -> Result<Notary, CertificateError> {
    current_notary_in(&notary_dir()?, amount, chrono::Utc::now())
}

fn current_notary_in(
    dir: &Path,
    amount: u64,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Notary, CertificateError> {
    load_notary(dir, amount, current_period(dir, now))
}

/// The notary pair a PaymentIntent was quoted from, given its
/// [`NOTARY_PERIOD_METADATA_KEY`] value. A PaymentIntent without one was quoted
/// from the flat files, including every one created before the schedule.
pub(crate) fn quoted_notary(amount: u64, period: Option<&str>) -> Result<Notary, CertificateError> {
    quoted_notary_in(&notary_dir()?, amount, period)
}

fn quoted_notary_in(
    dir: &Path,
    amount: u64,
    period: Option<&str>,
) -> Result<Notary, CertificateError> {
    let period = match period {
        None | Some("") => None,
        Some(p) if is_period(p) => Some(p.to_string()),
        Some(p) => {
            return Err(CertificateError::KeyError(format!(
                "invalid {} on PaymentIntent: {:?}",
                NOTARY_PERIOD_METADATA_KEY, p
            )))
        }
    };
    load_notary(dir, amount, period)
}

fn load_notary(
    dir: &Path,
    amount: u64,
    period: Option<String>,
) -> Result<Notary, CertificateError> {
    let (certificate, signing_key) = match &period {
        Some(p) => read_pair(&dir.join(p), amount)?,
        None => read_pair(dir, amount)?,
    };
    Ok(Notary {
        certificate,
        signing_key,
        period,
    })
}

fn read_pair(
    dir: &Path,
    amount: u64,
) -> Result<(NotaryCertificateV1, RSASigningKey), CertificateError> {
    let scheme = pick_scheme(dir, amount);

    let cert_path = dir.join(scheme.cert_filename(amount));
    let cert = NotaryCertificateV1::from_file(&cert_path).map_err(|e| {
        CertificateError::KeyError(format!(
            "Unable to read notary certificate from {}: {}",
            cert_path.display(),
            e
        ))
    })?;

    let signing_key_path = dir.join(scheme.signing_key_filename(amount));
    let signing_key = RSASigningKey::from_file(&signing_key_path).map_err(|e| {
        CertificateError::KeyError(format!(
            "Unable to read notary signing key from {}: {}",
            signing_key_path.display(),
            e
        ))
    })?;
    Ok((cert, signing_key))
}

pub(crate) fn sign_with_notary_key(
    blinded_ghostkey: &BlindedMessage,
    notary_signing_key: &RSASigningKey,
) -> Result<BlindSignature, CertificateError> {
    let options = Options::default();

    let blind_sig = notary_signing_key
        .blind_sign(&mut OsRng, blinded_ghostkey, &options)
        .map_err(|e| CertificateError::MiscError(format!("Failed to blind sign: {}", e)))?;

    Ok(blind_sig)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::tempdir;

    fn at(year: i32, month: u32) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(year, month, 15, 12, 0, 0)
            .unwrap()
    }

    /// Write a real notary pair for `amount` into `dir`, with `info` as its
    /// info string so a test can tell which pair it got back.
    fn write_pair(dir: &Path, amount: u64, info: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let master = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let (cert, key) = NotaryCertificateV1::new(&master, &info.to_string()).unwrap();
        cert.to_file(&dir.join(NamingScheme::Notary.cert_filename(amount)))
            .unwrap();
        key.to_file(&dir.join(NamingScheme::Notary.signing_key_filename(amount)))
            .unwrap();
    }

    fn assert_pair_matches(notary: &Notary) {
        assert_eq!(
            notary.signing_key.public_key().unwrap().to_der().unwrap(),
            notary
                .certificate
                .payload
                .notary_verifying_key
                .to_der()
                .unwrap(),
            "signing key does not belong to the certificate it was loaded with"
        );
    }

    #[test]
    fn is_period_accepts_only_real_months() {
        assert!(is_period("2026-10"));
        assert!(is_period("2036-12"));
        for bad in [
            "2026-13", "2026-00", "2026-1", "26-10", "2026_10", "../2026", "2026-10/", "",
        ] {
            assert!(!is_period(bad), "{bad:?} accepted");
        }
    }

    #[test]
    fn current_period_is_newest_month_not_in_the_future() {
        let dir = tempdir().unwrap();
        for m in ["2026-09", "2026-10", "2026-11", "notes"] {
            std::fs::create_dir(dir.path().join(m)).unwrap();
        }
        // A file, not a directory, must not count.
        touch(&dir.path().join("2026-12"));

        assert_eq!(
            current_period(dir.path(), at(2026, 10)).as_deref(),
            Some("2026-10")
        );
        // Schedule ran out: keep issuing from the newest month rather than fail.
        assert_eq!(
            current_period(dir.path(), at(2027, 3)).as_deref(),
            Some("2026-11")
        );
        // Only future months: no schedule in force yet, use the flat files.
        assert_eq!(current_period(dir.path(), at(2026, 8)), None);
    }

    #[test]
    fn current_period_without_schedule_is_flat() {
        let dir = tempdir().unwrap();
        assert_eq!(current_period(dir.path(), at(2026, 10)), None);
        assert_eq!(
            current_period(&dir.path().join("missing"), at(2026, 10)),
            None
        );
    }

    #[test]
    fn current_notary_loads_this_months_pair() {
        let dir = tempdir().unwrap();
        write_pair(dir.path(), 20, "flat");
        write_pair(&dir.path().join("2026-10"), 20, "2026-10");

        let n = current_notary_in(dir.path(), 20, at(2026, 10)).unwrap();
        assert_eq!(n.period.as_deref(), Some("2026-10"));
        assert_eq!(n.certificate.payload.info, "2026-10");
        assert_pair_matches(&n);

        let n = current_notary_in(dir.path(), 20, at(2026, 9)).unwrap();
        assert_eq!(n.period, None);
        assert_eq!(n.certificate.payload.info, "flat");
        assert_pair_matches(&n);
    }

    #[test]
    fn current_notary_fails_rather_than_mixing_periods() {
        // This month exists but lacks the tier: an error, not an older pair,
        // so a broken schedule is loud and the quote never names a period
        // whose files it did not come from.
        let dir = tempdir().unwrap();
        write_pair(&dir.path().join("2026-09"), 20, "2026-09");
        std::fs::create_dir(dir.path().join("2026-10")).unwrap();
        assert!(current_notary_in(dir.path(), 20, at(2026, 10)).is_err());
    }

    #[test]
    fn quoted_notary_uses_the_recorded_period_whatever_the_date() {
        // Quoted in October, signed after the schedule has moved on: must
        // still be October's pair, the one the browser blinded against.
        let dir = tempdir().unwrap();
        write_pair(dir.path(), 5, "flat");
        write_pair(&dir.path().join("2026-10"), 5, "2026-10");
        write_pair(&dir.path().join("2026-11"), 5, "2026-11");

        let n = quoted_notary_in(dir.path(), 5, Some("2026-10")).unwrap();
        assert_eq!(n.certificate.payload.info, "2026-10");
        assert_pair_matches(&n);

        // No period recorded (quoted from the flat files, or before the
        // schedule existed): the flat pair, even though months exist.
        for none in [None, Some("")] {
            let n = quoted_notary_in(dir.path(), 5, none).unwrap();
            assert_eq!(n.certificate.payload.info, "flat");
            assert_pair_matches(&n);
        }
    }

    #[test]
    fn quote_then_sign_after_rollover_uses_the_same_pair() {
        // The protocol invariant end to end: the period current_notary records
        // at quote time, fed back to quoted_notary after the month has turned,
        // yields the very certificate the donor was quoted.
        let dir = tempdir().unwrap();
        write_pair(&dir.path().join("2026-10"), 5, "2026-10");
        write_pair(&dir.path().join("2026-11"), 5, "2026-11");

        let last_second = chrono::Utc
            .with_ymd_and_hms(2026, 10, 31, 23, 59, 59)
            .unwrap();
        let quoted = current_notary_in(dir.path(), 5, last_second).unwrap();
        assert_eq!(quoted.period.as_deref(), Some("2026-10"));

        let first_second = chrono::Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
        assert_eq!(
            current_notary_in(dir.path(), 5, first_second)
                .unwrap()
                .period
                .as_deref(),
            Some("2026-11")
        );

        let signed = quoted_notary_in(dir.path(), 5, quoted.period.as_deref()).unwrap();
        assert_eq!(
            signed.certificate.to_base64().unwrap(),
            quoted.certificate.to_base64().unwrap()
        );
        assert_pair_matches(&signed);
    }

    #[test]
    fn quoted_notary_rejects_bad_or_missing_periods() {
        let dir = tempdir().unwrap();
        write_pair(dir.path(), 5, "flat");
        assert!(quoted_notary_in(dir.path(), 5, Some("../5")).is_err());
        assert!(quoted_notary_in(dir.path(), 5, Some("2026-13")).is_err());
        // Well-formed but never generated or since deleted.
        assert!(quoted_notary_in(dir.path(), 5, Some("2026-10")).is_err());
    }

    fn touch(path: &Path) {
        std::fs::write(path, b"placeholder").unwrap();
    }

    #[test]
    fn pick_scheme_prefers_notary_when_both_complete_pairs_exist() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("notary_certificate_20.pem"));
        touch(&dir.path().join("notary_signing_key_20.pem"));
        touch(&dir.path().join("delegate_certificate_20.pem"));
        touch(&dir.path().join("delegate_signing_key_20.pem"));
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn pick_scheme_falls_back_to_legacy_when_notary_absent() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("delegate_certificate_20.pem"));
        touch(&dir.path().join("delegate_signing_key_20.pem"));
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::LegacyDelegate);
    }

    #[test]
    fn pick_scheme_refuses_mismatched_partial_notary_pair() {
        // Only the notary cert has been renamed; signing key is still legacy.
        // Must NOT return LegacyDelegate (would pair a notary cert with a
        // legacy-named signing key for the wrong amount or worse). Instead
        // return Notary so the caller surfaces a clean "signing key not
        // found" error against the canonical filename.
        let dir = tempdir().unwrap();
        touch(&dir.path().join("notary_certificate_20.pem"));
        touch(&dir.path().join("delegate_signing_key_20.pem"));
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn pick_scheme_refuses_mismatched_partial_legacy_pair() {
        let dir = tempdir().unwrap();
        touch(&dir.path().join("delegate_certificate_20.pem"));
        touch(&dir.path().join("notary_signing_key_20.pem"));
        // Neither a complete notary pair nor a complete legacy pair;
        // default to canonical so the failure message points at the new name.
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn pick_scheme_missing_directory_defaults_to_notary() {
        let dir = tempdir().unwrap();
        assert_eq!(pick_scheme(dir.path(), 20), NamingScheme::Notary);
    }

    #[test]
    fn naming_scheme_filenames_are_exactly_as_documented() {
        assert_eq!(
            NamingScheme::Notary.cert_filename(20),
            "notary_certificate_20.pem"
        );
        assert_eq!(
            NamingScheme::Notary.signing_key_filename(20),
            "notary_signing_key_20.pem"
        );
        assert_eq!(
            NamingScheme::LegacyDelegate.cert_filename(20),
            "delegate_certificate_20.pem"
        );
        assert_eq!(
            NamingScheme::LegacyDelegate.signing_key_filename(20),
            "delegate_signing_key_20.pem"
        );
    }
}
