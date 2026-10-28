//! The notary certificate's `info` string.
//!
//! Current notary keys carry JSON written by `rust/cli/generate_notary_keys.sh`:
//!
//! ```text
//! {"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-30 15:39:26"}
//! ```
//!
//! The `delegate-key-created` name predates the notary rename and is frozen:
//! it is inside the master-signed bytes of every certificate issued. Older
//! test certificates carry `donation_amount:<n>` instead; [`NotaryInfo::parse`]
//! refuses that, since it has no creation date.

use serde::Deserialize;

use crate::Error;

/// A parsed notary `info` string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotaryInfo {
    /// What the notary witnessed. `"freenet-donation"` for every notary key
    /// issued so far. Not checked here: a caller that means "a donor" should
    /// check it, so a future kind of notary is not taken for one.
    pub action: String,
    /// The donation tier, in US dollars.
    pub amount: u64,
    /// When the notary key was created, as written:
    /// `"YYYY-MM-DD HH:MM:SS"` UTC.
    pub delegate_key_created: String,
    /// The year and month of `delegate_key_created`, which is what a
    /// certificate reveals about when its donation was made.
    pub created_month: YearMonth,
}

/// A calendar month.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    /// Four-digit year.
    pub year: u16,
    /// 1 to 12.
    pub month: u8,
}

#[derive(Deserialize)]
struct InfoJson {
    action: String,
    amount: u64,
    #[serde(rename = "delegate-key-created")]
    delegate_key_created: String,
}

impl NotaryInfo {
    /// Parse the JSON `info` of a current notary certificate.
    ///
    /// Requires `action` (a string), `amount` (a non-negative integer) and
    /// `delegate-key-created` (a string starting `YYYY-MM-DD`); other fields
    /// are ignored. Only the date's shape is checked (month 1 to 12, day 1 to
    /// 31), not the calendar or the time after it. The string is signed by the
    /// master key, so this is a reader, not a validator.
    pub fn parse(info: &str) -> Result<Self, Error> {
        let json: InfoJson = serde_json::from_str(info).map_err(|_| Error::Info)?;
        let created_month = parse_month(&json.delegate_key_created).ok_or(Error::Info)?;
        Ok(NotaryInfo {
            action: json.action,
            amount: json.amount,
            delegate_key_created: json.delegate_key_created,
            created_month,
        })
    }
}

fn parse_month(created: &str) -> Option<YearMonth> {
    let b = created.as_bytes();
    let digits = |r: core::ops::Range<usize>| -> Option<u16> {
        let s = b.get(r)?;
        if !s.iter().all(u8::is_ascii_digit) {
            return None;
        }
        Some(s.iter().fold(0u16, |acc, d| acc * 10 + u16::from(d - b'0')))
    };
    if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
        return None;
    }
    let year = digits(0..4)?;
    let month = digits(5..7)?;
    let day = digits(8..10)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if b.len() > 10 && b[10] != b' ' {
        return None;
    }
    Some(YearMonth {
        year,
        month: month as u8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_generator_format() {
        let info = NotaryInfo::parse(
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-30 15:39:26"}"#,
        )
        .unwrap();
        assert_eq!(info.action, "freenet-donation");
        assert_eq!(info.amount, 20);
        assert_eq!(info.delegate_key_created, "2024-07-30 15:39:26");
        assert_eq!(
            info.created_month,
            YearMonth {
                year: 2024,
                month: 7
            }
        );
    }

    #[test]
    fn ignores_unknown_fields_and_field_order() {
        let info = NotaryInfo::parse(
            r#"{"delegate-key-created":"2026-10-01 00:00:00","extra":[1],"amount":5,"action":"x"}"#,
        )
        .unwrap();
        assert_eq!(info.amount, 5);
        assert_eq!(info.created_month.month, 10);
    }

    #[test]
    fn refuses_what_it_cannot_read() {
        for bad in [
            "donation_amount:20",
            "",
            r#"{"action":"freenet-donation","amount":20}"#,
            r#"{"action":"freenet-donation","amount":-1,"delegate-key-created":"2024-07-30 15:39:26"}"#,
            r#"{"action":"freenet-donation","amount":"20","delegate-key-created":"2024-07-30 15:39:26"}"#,
            r#"{"action":"freenet-donation","amount":20.5,"delegate-key-created":"2024-07-30 15:39:26"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-13-30 15:39:26"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-00-30"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-7-30"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024/07/30"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-30T15:39:26"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"+024-07-30"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-00 00:00:00"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-32 00:00:00"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-30X15:39:26"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"２０２４-07-30"}"#,
            r#"{"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-3"}"#,
            r#"{"action":"freenet-donation","amount":18446744073709551616,"delegate-key-created":"2024-07-30"}"#,
            r#"{"action":"freenet-donation","amount":20,"amount":21,"delegate-key-created":"2024-07-30"}"#,
        ] {
            assert_eq!(NotaryInfo::parse(bad), Err(Error::Info), "{bad}");
        }
    }

    #[test]
    fn date_without_time_is_accepted() {
        let info = NotaryInfo::parse(
            r#"{"action":"freenet-donation","amount":1,"delegate-key-created":"2026-01-01"}"#,
        )
        .unwrap();
        assert_eq!(
            info.created_month,
            YearMonth {
                year: 2026,
                month: 1
            }
        );
    }
}
