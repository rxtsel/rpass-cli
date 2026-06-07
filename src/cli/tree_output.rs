use std::collections::BTreeMap;
use std::fmt;

const ROOT_LABEL: &str = "Password Store";
const BRANCH: &str = "\u{251c}\u{2500}\u{2500} ";
const LAST_BRANCH: &str = "\u{2514}\u{2500}\u{2500} ";
const INDENT: &str = "    ";
const CONTINUATION: &str = "\u{2502}   ";

#[derive(Debug, Default)]
pub struct EntryTree {
    root: TreeNode,
}

impl EntryTree {
    pub fn from_entries(entries: &[String]) -> Self {
        let mut tree = Self::default();

        for entry in entries {
            tree.insert(entry);
        }

        tree
    }

    fn insert(&mut self, entry: &str) {
        self.root.insert(entry.split('/'));
    }
}

impl fmt::Display for EntryTree {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "{ROOT_LABEL}")?;
        self.root.write_children(formatter, "")
    }
}

#[derive(Debug, Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
}

impl TreeNode {
    fn insert<'entry>(&mut self, mut segments: impl Iterator<Item = &'entry str>) {
        let Some(segment) = segments.next() else {
            return;
        };

        self.children
            .entry(segment.to_owned())
            .or_default()
            .insert(segments);
    }

    fn write_children(&self, formatter: &mut fmt::Formatter<'_>, prefix: &str) -> fmt::Result {
        let child_count = self.children.len();

        for (index, (name, child)) in self.children.iter().enumerate() {
            let is_last_child = index + 1 == child_count;
            child.write(formatter, prefix, name, is_last_child)?;
        }

        Ok(())
    }

    fn write(
        &self,
        formatter: &mut fmt::Formatter<'_>,
        prefix: &str,
        name: &str,
        is_last_child: bool,
    ) -> fmt::Result {
        let branch = branch_for_position(is_last_child);
        writeln!(formatter, "{prefix}{branch}{name}")?;

        let child_prefix = child_prefix(prefix, is_last_child);
        self.write_children(formatter, &child_prefix)
    }
}

fn branch_for_position(is_last_child: bool) -> &'static str {
    if is_last_child { LAST_BRANCH } else { BRANCH }
}

fn child_prefix(prefix: &str, is_last_child: bool) -> String {
    let suffix = if is_last_child { INDENT } else { CONTINUATION };
    format!("{prefix}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::EntryTree;

    #[test]
    fn renders_empty_store() {
        let entries = Vec::new();
        let tree = EntryTree::from_entries(&entries);

        assert_eq!(tree.to_string(), "Password Store\n");
    }

    #[test]
    fn renders_nested_entries_as_tree() {
        let entries = vec![
            "email/personal".to_string(),
            "email/work".to_string(),
            "github".to_string(),
        ];

        let tree = EntryTree::from_entries(&entries);

        assert_eq!(
            tree.to_string(),
            "\
Password Store
\u{251c}\u{2500}\u{2500} email
\u{2502}   \u{251c}\u{2500}\u{2500} personal
\u{2502}   \u{2514}\u{2500}\u{2500} work
\u{2514}\u{2500}\u{2500} github
"
        );
    }
}
