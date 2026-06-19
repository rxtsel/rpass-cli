use serde::Serialize;
use totp_rs::{TOTP, TotpUrlError};
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
            .map_err(|e| PasswordStoreError::InvalidOtpUri(totp_error_message(e)))?;

        Ok(Self {
            code: totp.generate(timestamp),
            remaining_seconds: remaining_seconds(timestamp, totp.step),
            period: totp.step,
        })
    }
}

fn totp_error_message(err: TotpUrlError) -> String {
    match err {
        TotpUrlError::Url(e) => format!("invalid URL: {e}"),
        TotpUrlError::Scheme(s) => format!("invalid scheme: {s}"),
        TotpUrlError::Host(s) => format!("invalid host: {s}"),
        TotpUrlError::Secret(_) => "invalid base32 secret".to_string(),
        TotpUrlError::SecretSize(n) => format!("secret too short: {n} bits"),
        TotpUrlError::Algorithm(s) => format!("unknown algorithm: {s}"),
        TotpUrlError::Digits(s) => format!("invalid digits: {s}"),
        TotpUrlError::DigitsNumber(n) => format!("invalid digits count: {n}"),
        TotpUrlError::Step(s) => format!("invalid period: {s}"),
        TotpUrlError::Issuer(s) => format!("issuer contains colon: {s}"),
        TotpUrlError::IssuerDecoding(s) => format!("could not decode issuer: {s}"),
        TotpUrlError::IssuerMistmatch(a, b) => format!("issuer mismatch: {a} != {b}"),
        TotpUrlError::AccountName(s) => format!("invalid account name: {s}"),
        TotpUrlError::AccountNameDecoding(s) => format!("could not decode account name: {s}"),
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
    let mut url = Url::parse(otp_uri)
        .map_err(|e| PasswordStoreError::InvalidOtpUri(format!("invalid URL: {e}")))?;

    let path_has_issuer = urlencoding::decode(url.path().trim_start_matches('/'))
        .ok()
        .is_some_and(|decoded| decoded.contains(':'));

    let query_pairs = url
        .query_pairs()
        .filter(|(key, _)| !(path_has_issuer && key.eq_ignore_ascii_case("issuer")))
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
    use super::{DecryptedEntry, OtpCode, normalize_otp_uri, remaining_seconds, totp_error_message};
    use totp_rs::TotpUrlError;

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
    fn generates_code_with_conflicting_issuer_in_query() {
        let entry = DecryptedEntry::parse(
            "\
secret
otpauth://totp/Politecnico+Grancolombiano:user@example.com?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&issuer=Microsoft
",
        );

        let code = OtpCode::generate_at(&entry, 1000).expect("otp");

        assert_eq!(code.code.len(), 6);
        assert_eq!(code.code, "804420");
    }

    #[test]
    fn preserves_non_secret_query_parameters() {
        let normalized = normalize_otp_uri(
            "otpauth://totp/Stripe:test?secret=eq2gkb3bljy7hansqf2kmqb7&issuer=Stripe&period=60",
        )
        .expect("normalized uri");

        assert!(normalized.contains("secret=EQ2GKB3BLJY7HANSQF2KMQB7"));
        assert!(normalized.contains("period=60"));
        assert!(normalized.contains("/Stripe:test"));
        // issuer query param dropped because path already has issuer
        assert!(!normalized.contains("issuer=Stripe"));
    }

    #[test]
    fn drops_issuer_param_when_path_has_issuer() {
        let normalized = normalize_otp_uri(
            "otpauth://totp/Politecnico+Grancolombiano:user@example.com?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&issuer=Microsoft",
        )
        .expect("normalized uri");

        assert!(normalized.contains("secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"));
        assert!(normalized.contains("/Politecnico+Grancolombiano:user@example.com"));
        assert!(!normalized.contains("issuer=Microsoft"));
    }

    #[test]
    fn keeps_issuer_param_when_path_has_no_issuer() {
        let normalized = normalize_otp_uri(
            "otpauth://totp/user@example.com?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&issuer=GitHub",
        )
        .expect("normalized uri");

        assert!(normalized.contains("issuer=GitHub"));
    }

    #[test]
    fn drops_issuer_param_when_path_has_percent_encoded_colon() {
        let normalized = normalize_otp_uri(
            "otpauth://totp/Politecnico+Grancolombiano%3auser@example.com?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&issuer=Microsoft",
        )
        .expect("normalized uri");

        assert!(normalized.contains("secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ"));
        assert!(!normalized.contains("issuer=Microsoft"));
    }

    #[test]
    fn generates_code_with_percent_encoded_colon_in_path() {
        let entry = DecryptedEntry::parse(
            "\
secret
otpauth://totp/Politecnico+Grancolombiano%3auser@example.com?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ&issuer=Microsoft
",
        );

        let code = OtpCode::generate_at(&entry, 1000).expect("otp");

        assert_eq!(code.code.len(), 6);
        assert_eq!(code.code, "804420");
    }

    #[test]
    fn totp_error_message_does_not_leak_secret() {
        let err = TotpUrlError::Secret("supersecret123".to_string());
        let msg = totp_error_message(err);
        assert_eq!(msg, "invalid base32 secret");
    }

    #[test]
    fn totp_error_message_variants() {
        let cases = vec![
            (TotpUrlError::SecretSize(80), "secret too short: 80 bits"),
            (
                TotpUrlError::Algorithm("MD5".to_string()),
                "unknown algorithm: MD5",
            ),
            (
                TotpUrlError::Host("hotp".to_string()),
                "invalid host: hotp",
            ),
            (
                TotpUrlError::Scheme("https".to_string()),
                "invalid scheme: https",
            ),
            (
                TotpUrlError::Digits("abc".to_string()),
                "invalid digits: abc",
            ),
            (TotpUrlError::DigitsNumber(5), "invalid digits count: 5"),
            (
                TotpUrlError::Step("xyz".to_string()),
                "invalid period: xyz",
            ),
            (
                TotpUrlError::Issuer("Iss:uer".to_string()),
                "issuer contains colon: Iss:uer",
            ),
            (
                TotpUrlError::IssuerDecoding("iss%uer".to_string()),
                "could not decode issuer: iss%uer",
            ),
            (
                TotpUrlError::IssuerMistmatch("Google".to_string(), "Github".to_string()),
                "issuer mismatch: Google != Github",
            ),
            (
                TotpUrlError::AccountName("Laziz:".to_string()),
                "invalid account name: Laziz:",
            ),
            (
                TotpUrlError::AccountNameDecoding("Laz%iz".to_string()),
                "could not decode account name: Laz%iz",
            ),
            (
                TotpUrlError::Url(url::ParseError::EmptyHost),
                "invalid URL: empty host",
            ),
        ];

        for (err, expected) in cases {
            assert_eq!(
                totp_error_message(err),
                expected,
                "mismatch for variant {expected}",
            );
        }
    }
}
