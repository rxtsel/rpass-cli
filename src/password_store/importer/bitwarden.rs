use std::collections::HashMap;

use serde_json::Value;

use super::{ImportEntry, ImportError, Importer};
use crate::password_store::EntryField;

#[derive(Debug)]
pub struct BitwardenImporter;

impl Importer for BitwardenImporter {
    fn parse(&self, data: &str) -> Result<Vec<ImportEntry>, ImportError> {
        let root: Value = serde_json::from_str(data)?;

        if root.get("encrypted").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Err(ImportError::EncryptedFile);
        }

        let items = root
            .get("items")
            .and_then(|v| v.as_array())
            .ok_or(ImportError::NoEntries)?;

        if items.is_empty() {
            return Err(ImportError::NoEntries);
        }

        let folders = build_folder_map(&root);
        let mut entries = Vec::new();

        for item in items {
            if let Some(entry) = parse_item(item, &folders) {
                entries.push(entry);
            }
        }

        if entries.is_empty() {
            return Err(ImportError::NoEntries);
        }

        Ok(entries)
    }
}

fn build_folder_map(root: &Value) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for key in &["folders", "collections"] {
        let Some(groups) = root.get(*key).and_then(|v| v.as_array()) else {
            continue;
        };

        for group in groups {
            let id = group.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_owned();
            let name = group.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_owned();
            if !id.is_empty() {
                map.insert(id, name);
            }
        }
    }

    map
}

fn parse_item(item: &Value, folders: &HashMap<String, String>) -> Option<ImportEntry> {
    let name = item.get("name")?.as_str()?.trim().to_owned();
    if name.is_empty() {
        return None;
    }

    let folder = resolve_folder(item, folders);
    let login = item.get("login");

    let password = login
        .and_then(|l| l.get("password"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let username = login
        .and_then(|l| l.get("username"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let totp = login
        .and_then(|l| l.get("totp"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let uris = login
        .and_then(|l| l.get("uris"))
        .and_then(|v| v.as_array());

    let mut fields = Vec::new();

    if let Some(ref u) = username {
        fields.push(EntryField {
            name: "username".to_owned(),
            value: u.clone(),
        });
    }

    if let Some(uris) = uris {
        for (i, uri_obj) in uris.iter().enumerate() {
            let uri = uri_obj
                .get("uri")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty());
            if let Some(uri) = uri {
                let key = if i == 0 { "url".to_owned() } else { format!("url{}", i + 1) };
                fields.push(EntryField {
                    name: key,
                    value: uri.to_owned(),
                });
            }
        }
    }

    let item_type = item.get("type").and_then(|v| v.as_i64()).unwrap_or(1);
    if item_type == 3 {
        if let Some(card) = item.get("card") {
            flatten_object(card, &mut fields);
        }
    } else if item_type == 4 && let Some(identity) = item.get("identity") {
        flatten_object(identity, &mut fields);
    }

    if let Some(fields_arr) = item.get("fields").and_then(|v| v.as_array()) {
        for field in fields_arr {
            let fname = field.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let fvalue = field.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if !fname.is_empty() && !fvalue.is_empty() {
                fields.push(EntryField {
                    name: fname.to_owned(),
                    value: fvalue.to_owned(),
                });
            }
        }
    }

    let otp_uri = totp.map(|t| normalize_totp(&t, &name));

    let notes = item
        .get("notes")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    Some(ImportEntry {
        name,
        password,
        fields,
        otp_uri,
        notes,
        folder,
    })
}

fn resolve_folder(item: &Value, folders: &HashMap<String, String>) -> Option<String> {
    if let Some(folder_id) = item.get("folderId").and_then(|v| v.as_str())
        && !folder_id.is_empty()
    {
        return folders.get(folder_id).filter(|n| !n.is_empty()).cloned();
    }

    if let Some(collection_ids) = item.get("collectionIds").and_then(|v| v.as_array()) {
        for coll_id in collection_ids {
            if let Some(id) = coll_id.as_str()
                && !id.is_empty()
                && let Some(name) = folders.get(id)
                && !name.is_empty()
            {
                return Some(name.clone());
            }
        }
    }

    None
}

fn flatten_object(obj: &Value, fields: &mut Vec<EntryField>) {
    let Some(map) = obj.as_object() else {
        return;
    };

    for (k, v) in map {
        if let Some(s) = v.as_str()
            && !s.is_empty()
        {
            fields.push(EntryField {
                name: k.clone(),
                value: s.to_owned(),
            });
        }
    }
}

fn normalize_totp(totp: &str, name: &str) -> String {
    if totp.starts_with("otpauth://") {
        totp.to_owned()
    } else {
        format!("otpauth://totp/{name}?secret={totp}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_individual_vault() {
        let json = r#"{
            "encrypted": false,
            "folders": [
                { "id": "f1", "name": "Social" }
            ],
            "items": [
                {
                    "id": "i1",
                    "folderId": "f1",
                    "type": 1,
                    "name": "Twitter",
                    "notes": "My Twitter account",
                    "favorite": false,
                    "fields": [
                        { "name": "handle", "value": "@me", "type": 0 }
                    ],
                    "login": {
                        "uris": [{ "match": null, "uri": "https://twitter.com" }],
                        "username": "me@example.com",
                        "password": "secret123",
                        "totp": "JBSWY3DPEHPK3PXP"
                    }
                }
            ]
        }"#;

        let importer = BitwardenImporter;
        let entries = importer.parse(json).expect("parse");
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry.name, "Twitter");
        assert_eq!(entry.folder.as_deref(), Some("Social"));
        assert_eq!(entry.password.as_deref(), Some("secret123"));
        assert!(entry.fields.iter().any(|f| f.name == "username" && f.value == "me@example.com"));
        assert!(entry.fields.iter().any(|f| f.name == "url" && f.value == "https://twitter.com"));
        assert!(entry.fields.iter().any(|f| f.name == "handle" && f.value == "@me"));
        assert!(entry.otp_uri.as_deref().unwrap().starts_with("otpauth://"));
        assert_eq!(entry.notes.as_deref(), Some("My Twitter account"));
    }

    #[test]
    fn rejects_encrypted_export() {
        let json = r#"{"encrypted": true, "items": []}"#;
        let importer = BitwardenImporter;
        assert!(matches!(importer.parse(json), Err(ImportError::EncryptedFile)));
    }

    #[test]
    fn handles_no_folder() {
        let json = r#"{
            "encrypted": false,
            "items": [
                {
                    "id": "i1",
                    "folderId": null,
                    "type": 1,
                    "name": "Direct Login",
                    "login": { "username": "user", "password": "pass" }
                }
            ]
        }"#;

        let importer = BitwardenImporter;
        let entries = importer.parse(json).expect("parse");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].folder.is_none());
    }

    #[test]
    fn handles_secure_note() {
        let json = r#"{
            "encrypted": false,
            "items": [
                {
                    "id": "i1",
                    "type": 2,
                    "name": "My Note",
                    "notes": "This is a secure note content"
                }
            ]
        }"#;

        let importer = BitwardenImporter;
        let entries = importer.parse(json).expect("parse");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].password.is_none());
        assert_eq!(entries[0].notes.as_deref(), Some("This is a secure note content"));
    }

    #[test]
    fn handles_card() {
        let json = r#"{
            "encrypted": false,
            "items": [
                {
                    "id": "i1",
                    "type": 3,
                    "name": "My Card",
                    "card": {
                        "brand": "Visa",
                        "number": "4111111111111111",
                        "cardholderName": "John Doe"
                    }
                }
            ]
        }"#;

        let importer = BitwardenImporter;
        let entries = importer.parse(json).expect("parse");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].fields.iter().any(|f| f.name == "brand" && f.value == "Visa"));
    }

    #[test]
    fn handles_organization_vault() {
        let json = r#"{
            "encrypted": false,
            "collections": [
                { "id": "c1", "organizationId": "o1", "name": "Team Passwords" }
            ],
            "items": [
                {
                    "id": "i1",
                    "organizationId": "o1",
                    "collectionIds": ["c1"],
                    "type": 1,
                    "name": "Shared Login",
                    "login": { "username": "admin", "password": "admin123" }
                }
            ]
        }"#;

        let importer = BitwardenImporter;
        let entries = importer.parse(json).expect("parse");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].folder.as_deref(), Some("Team Passwords"));
    }

    #[test]
    fn normalizes_raw_totp_key() {
        assert_eq!(
            normalize_totp("JBSWY3DPEHPK3PXP", "test"),
            "otpauth://totp/test?secret=JBSWY3DPEHPK3PXP"
        );
    }

    #[test]
    fn preserves_full_otpauth_uri() {
        assert_eq!(
            normalize_totp("otpauth://totp/example?secret=ABC", "ignored"),
            "otpauth://totp/example?secret=ABC"
        );
    }

    #[test]
    fn handles_multiple_uris() {
        let json = r#"{
            "encrypted": false,
            "items": [
                {
                    "id": "i1",
                    "type": 1,
                    "name": "Multi URL",
                    "login": {
                        "uris": [
                            { "match": null, "uri": "https://primary.com" },
                            { "match": null, "uri": "https://backup.com" }
                        ],
                        "username": "user",
                        "password": "pass"
                    }
                }
            ]
        }"#;

        let importer = BitwardenImporter;
        let entries = importer.parse(json).expect("parse");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].fields.iter().any(|f| f.name == "url" && f.value == "https://primary.com"));
        assert!(entries[0].fields.iter().any(|f| f.name == "url2" && f.value == "https://backup.com"));
    }
}
