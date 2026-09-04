//! The file-tree sidebar model (`F2`).
//!
//! A flattened, lazily-expanded view of a directory. Only expanded directories
//! contribute their children to `rows`, so the cost is one `read_dir` per open
//! folder — cheap, but still filesystem I/O, so the app builds this **once** when
//! the sidebar opens and mutates it on expand/collapse/refresh, never per frame.
//!
//! The app owns the selection index and turns Enter / clicks into
//! open-file / toggle-directory.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One visible line in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub name: String,
    /// Nesting level below the root (root's direct children are 0).
    pub depth: usize,
    pub is_dir: bool,
    /// Directory only: whether its children are currently shown.
    pub expanded: bool,
    /// File only: ends in `.vul`.
    pub is_vul: bool,
}

pub struct FileTree {
    pub root: PathBuf,
    /// Absolute paths of directories the user has expanded.
    expanded: BTreeSet<PathBuf>,
    /// Currently-visible rows, rebuilt on every structural change.
    rows: Vec<Row>,
    pub show_hidden: bool,
}

impl FileTree {
    /// Read `root`'s immediate children (nothing expanded yet).
    pub fn new(root: &Path) -> Self {
        let mut t = Self {
            root: root.to_path_buf(),
            expanded: BTreeSet::new(),
            rows: Vec::new(),
            show_hidden: false,
        };
        t.rebuild();
        t
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn get(&self, idx: usize) -> Option<&Row> {
        self.rows.get(idx)
    }

    /// Re-read every open directory from disk, keeping the expanded set.
    pub fn refresh(&mut self) {
        self.rebuild();
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.rebuild();
    }

    /// Expand the directory at `idx` (no-op for files or already-open dirs).
    pub fn expand(&mut self, idx: usize) {
        if let Some(r) = self.rows.get(idx)
            && r.is_dir
            && !r.expanded
        {
            self.expanded.insert(r.path.clone());
            self.rebuild();
        }
    }

    /// Collapse the directory at `idx` (no-op for files or closed dirs).
    pub fn collapse(&mut self, idx: usize) {
        if let Some(r) = self.rows.get(idx)
            && r.is_dir
            && r.expanded
        {
            let prefix = r.path.clone();
            // Drop the folder and everything expanded beneath it.
            self.expanded
                .retain(|p| p != &prefix && !p.starts_with(&prefix));
            self.rebuild();
        }
    }

    /// Expand every ancestor directory of `path` and return its row index, if
    /// `path` is inside the root.
    pub fn reveal(&mut self, path: &Path) -> Option<usize> {
        let rel = path.strip_prefix(&self.root).ok()?;
        let mut acc = self.root.clone();
        for comp in rel.components() {
            acc.push(comp);
            if acc != path {
                self.expanded.insert(acc.clone());
            }
        }
        self.rebuild();
        self.rows.iter().position(|r| r.path == path)
    }

    /// Row index of the parent directory of `idx`, if any.
    pub fn parent_of(&self, idx: usize) -> Option<usize> {
        let depth = self.rows.get(idx)?.depth;
        if depth == 0 {
            return None;
        }
        self.rows[..idx].iter().rposition(|r| r.depth < depth)
    }

    fn rebuild(&mut self) {
        // Forget expanded dirs that have since disappeared.
        let mut rows = Vec::new();
        walk(&self.root, 0, &self.expanded, self.show_hidden, &mut rows);
        self.expanded.retain(|p| p.is_dir());
        self.rows = rows;
    }
}

fn walk(
    dir: &Path,
    depth: usize,
    expanded: &BTreeSet<PathBuf>,
    show_hidden: bool,
    out: &mut Vec<Row>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<(PathBuf, bool, String)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !show_hidden && name.starts_with('.') {
                return None;
            }
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            Some((e.path(), is_dir, name))
        })
        .collect();
    // Directories first, then case-insensitive by name.
    entries.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.2.to_lowercase().cmp(&b.2.to_lowercase()))
    });

    for (path, is_dir, name) in entries {
        let is_expanded = is_dir && expanded.contains(&path);
        out.push(Row {
            is_vul: !is_dir
                && path
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("vul")),
            name,
            depth,
            is_dir,
            expanded: is_expanded,
            path: path.clone(),
        });
        if is_expanded {
            walk(&path, depth + 1, expanded, show_hidden, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vulide_tree_{}_{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("main.vul"), "G\"hi\"\n").unwrap();
        fs::write(dir.join("notes.txt"), "x").unwrap();
        fs::write(dir.join(".hidden"), "x").unwrap();
        fs::write(dir.join("sub").join("inner.vul"), "Q\n").unwrap();
        dir
    }

    #[test]
    fn lists_dirs_first_then_files_and_hides_dotfiles() {
        let dir = fixture();
        let t = FileTree::new(&dir);
        let names: Vec<&str> = t.rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["sub", "main.vul", "notes.txt"]);
        assert!(t.rows()[0].is_dir);
        assert!(t.rows()[1].is_vul);
        assert!(!t.rows()[2].is_vul);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn expand_and_collapse_flatten_children() {
        let dir = fixture();
        let mut t = FileTree::new(&dir);
        assert_eq!(t.len(), 3);
        t.expand(0); // "sub"
        let names: Vec<&str> = t.rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["sub", "inner.vul", "main.vul", "notes.txt"]);
        assert_eq!(t.rows()[1].depth, 1);
        t.collapse(0);
        assert_eq!(t.len(), 3);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn show_hidden_reveals_dotfiles() {
        let dir = fixture();
        let mut t = FileTree::new(&dir);
        t.toggle_hidden();
        assert!(t.rows().iter().any(|r| r.name == ".hidden"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parent_of_walks_up_a_level() {
        let dir = fixture();
        let mut t = FileTree::new(&dir);
        t.expand(0);
        // row 1 is "inner.vul" at depth 1; its parent is row 0 ("sub").
        assert_eq!(t.parent_of(1), Some(0));
        assert_eq!(t.parent_of(0), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reveal_expands_ancestors_and_returns_the_row() {
        let dir = fixture();
        let mut t = FileTree::new(&dir);
        assert_eq!(t.len(), 3); // sub, main.vul, notes.txt — "sub" collapsed
        let idx = t.reveal(&dir.join("sub").join("inner.vul")).unwrap();
        assert_eq!(t.rows()[idx].name, "inner.vul");
        assert!(t.rows()[0].expanded, "ancestor 'sub' expanded");
        assert_eq!(t.reveal(Path::new("/outside/x.vul")), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_root_yields_no_rows() {
        let t = FileTree::new(Path::new("/vulide/definitely/not/here"));
        assert!(t.is_empty());
    }
}
