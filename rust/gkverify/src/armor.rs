//! PEM-style armour, as `ghostkey_lib`'s `Armorable` reads and writes it for
//! `GhostkeyCertificateV1`.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::Error;

/// The label `ghostkey_lib` writes. `Armorable::struct_name` derives it from
/// the type name `GhostkeyCertificateV1`.
const LABEL: &str = "GHOSTKEY_CERTIFICATE_V1";

/// The labels `ghostkey_lib` reads, in the order it tries them: the
/// versioned one, then the same without `_V1`. It has no legacy alias for
/// this type (only notary certificates and payloads have one).
const READ_LABELS: [&str; 2] = [LABEL, "GHOSTKEY_CERTIFICATE"];

/// The certificate in the first label whose block decodes, mirroring
/// `Armorable::from_armored_string`: the text after the first BEGIN line, up
/// to the END line if there is one, trimmed, with lines starting `-----`
/// dropped and the rest joined. A block that fails to decode, as base64 or as
/// a certificate, moves on to the next label.
pub(crate) fn decode(armored: &str) -> Result<crate::GhostkeyCertificate, Error> {
    for label in READ_LABELS {
        let begin = format!("-----BEGIN {label}-----");
        let end = format!("-----END {label}-----");
        let Some(block) = armored.split(begin.as_str()).nth(1) else {
            continue;
        };
        let Some(content) = block.split(end.as_str()).next() else {
            continue;
        };
        let base64: String = content
            .trim()
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect();
        let Ok(bytes) = STANDARD.decode(base64) else {
            continue;
        };
        if let Ok(cert) = crate::GhostkeyCertificate::from_cbor(&bytes) {
            return Ok(cert);
        }
    }
    Err(Error::Armor)
}

/// `Armorable::to_armored_string`: base64 wrapped at 64 columns.
pub(crate) fn encode(cbor: &[u8]) -> String {
    let base64 = STANDARD.encode(cbor);
    let mut out = format!("-----BEGIN {LABEL}-----\n");
    for chunk in base64.as_bytes().chunks(64) {
        out.push_str(core::str::from_utf8(chunk).expect("base64 is ASCII"));
        out.push('\n');
    }
    out.push_str(&format!("-----END {LABEL}-----\n"));
    out
}
