use serde::Serialize;
use totp_rs::TOTP;

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
        let totp = TOTP::from_url_unchecked(otp_uri)
            .map_err(|error| PasswordStoreError::InvalidOtpUri(error.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::{DecryptedEntry, OtpCode, remaining_seconds};

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
}
