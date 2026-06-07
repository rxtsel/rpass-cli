use std::fs;
use std::path::Path;

use tempfile::TempDir;

mod support;

use support::rpass;

#[test]
fn searches_entries_as_text() {
    let store = password_store_with_entries([
        "personal/openai.com.gpg",
        "work/OpenAI Admin.gpg",
        "personal/github.com.gpg",
    ]);

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "search",
            "openai",
        ])
        .assert()
        .success()
        .stdout(
            "\
Password Store
\u{251c}\u{2500}\u{2500} personal
\u{2502}   \u{2514}\u{2500}\u{2500} openai.com
\u{2514}\u{2500}\u{2500} work
    \u{2514}\u{2500}\u{2500} OpenAI Admin
",
        );
}

#[test]
fn searches_entries_as_json() {
    let store = password_store_with_entries([
        "personal/openai.com.gpg",
        "work/OpenAI Admin.gpg",
        "personal/github.com.gpg",
    ]);

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "search",
            "openai",
            "--json",
        ])
        .assert()
        .success()
        .stdout("[\n  \"personal/openai.com\",\n  \"work/OpenAI Admin\"\n]\n");
}

#[test]
fn returns_empty_output_when_text_search_has_no_matches() {
    let store = password_store_with_entries(["personal/github.com.gpg"]);

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "search",
            "openai",
        ])
        .assert()
        .success()
        .stdout("Password Store\n");
}

fn password_store_with_entries<const N: usize>(entries: [&str; N]) -> TempDir {
    let store = TempDir::new().expect("temp dir");

    for entry in entries {
        create_file(store.path().join(entry));
    }

    store
}

fn create_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, "").expect("file");
}
