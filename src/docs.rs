use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct DocFile {
    pub name: String,   // display name (stem)
    pub path: PathBuf,
}

/// A row in the sidebar tree — either a clickable file or a non-clickable directory header.
#[derive(Debug, Clone)]
pub enum SidebarEntry {
    File { file_idx: usize, depth: usize },
    Dir  { name: String,    depth: usize },
}

/// Scan the docs dir recursively.
/// Returns the flat navigation list and the ordered sidebar display tree.
pub fn scan(docs_dir: &Path) -> (Vec<DocFile>, Vec<SidebarEntry>) {
    let mut files   = Vec::new();
    let mut sidebar = Vec::new();
    scan_dir(docs_dir, 0, &mut files, &mut sidebar);
    (files, sidebar)
}

fn scan_dir(dir: &Path, depth: usize, files: &mut Vec<DocFile>, sidebar: &mut Vec<SidebarEntry>) {
    let Ok(entries) = fs::read_dir(dir) else { return };

    let mut md_files: Vec<DocFile> = Vec::new();
    let mut subdirs:  Vec<(String, PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if !name.starts_with('.') {
                subdirs.push((name, path));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string();
            md_files.push(DocFile { name, path });
        }
    }

    // index/readme first, then alphabetical
    md_files.sort_by(|a, b| {
        let a_pin = a.name.eq_ignore_ascii_case("index") || a.name.eq_ignore_ascii_case("readme");
        let b_pin = b.name.eq_ignore_ascii_case("index") || b.name.eq_ignore_ascii_case("readme");
        match (a_pin, b_pin) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    subdirs.sort_by(|(a, _), (b, _)| a.to_lowercase().cmp(&b.to_lowercase()));

    for f in md_files {
        sidebar.push(SidebarEntry::File { file_idx: files.len(), depth });
        files.push(f);
    }

    for (name, path) in subdirs {
        sidebar.push(SidebarEntry::Dir { name, depth });
        scan_dir(&path, depth + 1, files, sidebar);
    }
}

pub fn load(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| format!("# Error\n\nCould not load `{}`: {}", path.display(), e))
}

/// Resolve an href relative to current doc path → absolute path if it's a local .md file.
pub fn resolve_href(href: &str, current: &Path, docs_dir: &Path) -> Option<PathBuf> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return None; // external
    }
    // Strip fragment (#section) before resolving — "page.md#heading" should resolve to "page.md"
    let href = href.split('#').next().unwrap_or("");
    if href.is_empty() { return None; }
    let base = current.parent().unwrap_or(docs_dir);
    let candidate = base.join(href);
    if candidate.exists() {
        return Some(candidate);
    }
    // try adding .md
    let with_ext = base.join(format!("{}.md", href));
    if with_ext.exists() {
        return Some(with_ext);
    }
    None
}
