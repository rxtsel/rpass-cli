use serde::Serialize;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct DecryptedEntry {
    pub password: String,
    pub fields: Vec<EntryField>,
    pub otp_uri: Option<String>,
    pub extra_lines: Vec<String>,
}

impl DecryptedEntry {
    pub fn parse(content: &str) -> Self {
        let mut lines = content.lines();
        let password = lines.next().unwrap_or_default().to_owned();
        let mut fields = Vec::new();
        let mut otp_uri = None;
        let mut extra_lines = Vec::new();

        for line in lines {
            if is_otp_uri(line) {
                otp_uri = Some(line.to_owned());
                continue;
            }

            if let Some(field) = EntryField::parse(line) {
                fields.push(field);
                continue;
            }

            extra_lines.push(line.to_owned());
        }

        Self {
            password,
            fields,
            otp_uri,
            extra_lines,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct EntryField {
    pub name: String,
    pub value: String,
}

impl EntryField {
    fn parse(line: &str) -> Option<Self> {
        let (name, value) = line.split_once(':')?;
        let name = name.trim();

        if name.is_empty() {
            return None;
        }

        Some(Self {
            name: name.to_owned(),
            value: value.trim().to_owned(),
        })
    }
}

fn is_otp_uri(line: &str) -> bool {
    line.starts_with("otpauth://")
}

#[cfg(test)]
mod tests {
    use super::{DecryptedEntry, EntryField};

    #[test]
    fn parses_password_metadata_otp_and_extra_lines() {
        let entry = DecryptedEntry::parse(
            "\
secret
username: alice
url: https://example.com
otpauth://totp/example
recovery code: 123
notes without separator
",
        );

        assert_eq!(
            entry,
            DecryptedEntry {
                password: "secret".to_string(),
                fields: vec![
                    EntryField {
                        name: "username".to_string(),
                        value: "alice".to_string()
                    },
                    EntryField {
                        name: "url".to_string(),
                        value: "https://example.com".to_string()
                    },
                    EntryField {
                        name: "recovery code".to_string(),
                        value: "123".to_string()
                    }
                ],
                otp_uri: Some("otpauth://totp/example".to_string()),
                extra_lines: vec!["notes without separator".to_string()],
            }
        );
    }

    #[test]
    fn parses_empty_content_as_empty_password() {
        let entry = DecryptedEntry::parse("");

        assert_eq!(entry.password, "");
        assert!(entry.fields.is_empty());
    }
}
