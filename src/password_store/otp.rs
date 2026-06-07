use serde::Serialize;
use totp_rs::TOTP;
use url::Url;

use super::{DecryptedEntry, PasswordStoreError};

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct OtpCode {
    pub code: String,
    pub remaining_seconds: u64,
    pub period: u64,
}

impl OtpCode {
    pub fn generate_at(entry: &DecryptedEntry, timestamp: u64) -> Result<Self, PasswordStoreError> {
        let otp_uri = entry
            .otp_uri
            .as_deref()
            .ok_or(PasswordStoreError::OtpNotFound)?;
        let normalized_uri = normalize_otp_uri(otp_uri)?;
        let totp = TOTP::from_url_unchecked(&normalized_uri)
            .map_err(|_| PasswordStoreError::InvalidOtpUri)?;

        Ok(Self {
            code: totp.generate(timestamp),
            remaining_seconds: remaining_seconds(timestamp, totp.step),
            period: totp.step,
        })
    }
}

fn remaining_seconds(timestamp: u64, period: u64) -> u64 {
    let elapsed = timestamp % period;

    if elapsed == 0 {
        period
    } else {
        period - elapsed
    }
}

fn normalize_otp_uri(otp_uri: &str) -> Result<String, PasswordStoreError> {
    let mut url = Url::parse(otp_uri).map_err(|_| PasswordStoreError::InvalidOtpUri)?;
    let query_pairs = url
        .query_pairs()
        .map(|(key, value)| {
            let normalized_value = normalize_query_value(&key, &value);

            (key.into_owned(), normalized_value)
        })
        .collect::<Vec<_>>();

    url.query_pairs_mut().clear().extend_pairs(query_pairs);

    Ok(url.to_string())
}

fn normalize_query_value(key: &str, value: &str) -> String {
    if key.eq_ignore_ascii_case("secret") {
        value.to_ascii_uppercase()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{DecryptedEntry, OtpCode, normalize_otp_uri, remaining_seconds};

    #[test]
    fn generates_deterministic_totp_code() {
        let entry = DecryptedEntry::parse(
            "\
secret
otpauth://totp/GitHub:test?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ
",
        );

        let code = OtpCode::generate_at(&entry, 1000).expect("otp");

        assert_eq!(
            code,
            OtpCode {
                code: "804420".to_string(),
                remaining_seconds: 20,
                period: 30,
            }
        );
    }

    #[test]
    fn returns_full_period_at_step_boundary() {
        assert_eq!(remaining_seconds(60, 30), 30);
    }

    #[test]
    fn normalizes_lowercase_secret() {
        let entry = DecryptedEntry::parse(
            "\
secret
otpauth://totp/Stripe:test?secret=eq2gkb3bljy7hansqf2kmqb7
",
        );

        let code = OtpCode::generate_at(&entry, 1000).expect("otp");

        assert_eq!(code.code.len(), 6);
    }

    #[test]
    fn preserves_non_secret_query_parameters() {
        let normalized = normalize_otp_uri(
            "otpauth://totp/Stripe:test?secret=eq2gkb3bljy7hansqf2kmqb7&issuer=Stripe&period=60",
        )
        .expect("normalized uri");

        assert!(normalized.contains("secret=EQ2GKB3BLJY7HANSQF2KMQB7"));
        assert!(normalized.contains("issuer=Stripe"));
        assert!(normalized.contains("period=60"));
    }
}
