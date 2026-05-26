use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct DocFile {
    pub name: String,   // display name (stem)
    pub path: PathBuf,
}

pub fn scan(docs_dir: &Path) -> Vec<DocFile> {
    let mut files: Vec<DocFile> = Vec::new();

    let Ok(entries) = fs::read_dir(docs_dir) else { return files };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string();
            files.push(DocFile { name, path });
        }
    }

    // index first, then alphabetical
    files.sort_by(|a, b| {
        let a_idx = a.name.eq_ignore_ascii_case("index") || a.name.eq_ignore_ascii_case("readme");
        let b_idx = b.name.eq_ignore_ascii_case("index") || b.name.eq_ignore_ascii_case("readme");
        match (a_idx, b_idx) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    files
}

pub fn load(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| format!("# Error\n\nCould not load `{}`: {}", path.display(), e))
}

/// Resolve an href relative to current doc path → absolute path if it's a local .md file.
pub fn resolve_href(href: &str, current: &Path, docs_dir: &Path) -> Option<PathBuf> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return None; // external
    }
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
