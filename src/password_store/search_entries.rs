use super::{ListEntries, PasswordStore, PasswordStoreError};

pub struct SearchEntries<'store> {
    store: &'store PasswordStore,
}

impl<'store> SearchEntries<'store> {
    pub fn new(store: &'store PasswordStore) -> Self {
        Self { store }
    }

    pub fn execute(&self, query: &str) -> Result<Vec<String>, PasswordStoreError> {
        let query = SearchQuery::new(query);
        let entries = ListEntries::new(self.store).execute()?;

        Ok(entries
            .into_iter()
            .filter(|entry| query.matches(entry))
            .collect())
    }
}

struct SearchQuery {
    normalized: String,
}

impl SearchQuery {
    fn new(query: &str) -> Self {
        Self {
            normalized: query.to_lowercase(),
        }
    }

    fn matches(&self, entry: &str) -> bool {
        entry.to_lowercase().contains(&self.normalized)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::SearchEntries;
    use crate::password_store::{PasswordStore, StoreDirectory};

    #[test]
    fn finds_entries_by_case_insensitive_substring() {
        let temp_dir = TempDir::new().expect("temp dir");
        create_file(temp_dir.path().join("personal").join("openai.com.gpg"));
        create_file(temp_dir.path().join("work").join("OpenAI Admin.gpg"));
        create_file(temp_dir.path().join("personal").join("github.com.gpg"));
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");

        let entries = SearchEntries::new(&store)
            .execute("openai")
            .expect("entries");

        assert_eq!(entries, vec!["personal/openai.com", "work/OpenAI Admin"]);
    }

    #[test]
    fn returns_empty_results_when_query_does_not_match() {
        let temp_dir = TempDir::new().expect("temp dir");
        create_file(temp_dir.path().join("personal").join("github.com.gpg"));
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");

        let entries = SearchEntries::new(&store)
            .execute("openai")
            .expect("entries");

        assert!(entries.is_empty());
    }

    fn create_file(path: impl AsRef<std::path::Path>) {
        let path = path.as_ref();
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, "").expect("file");
    }
}
