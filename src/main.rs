#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod docs;
mod layout;
mod mermaid;
mod parser;
mod render;
mod search;
mod theme;

use std::path::PathBuf;
use windows::Win32::UI::HiDpi::*;

fn main() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Determine docs directory: prefer argv[1], else sibling "docs" folder next to exe.
    let docs_dir = std::env::args().nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let exe = std::env::current_exe().unwrap_or_default();
            exe.parent().unwrap_or(std::path::Path::new(".")).join("docs")
        });

    if !docs_dir.exists() {
        eprintln!("docs directory not found: {}", docs_dir.display());
        std::process::exit(1);
    }

    // Load shape registry — bundled defaults first, then per-doc-set overrides
    // from `<docs>/.shapes/*.shape`. Must happen before any mermaid block is
    // parsed (warm-up thread starts inside `render::run`).
    mermaid::shape_def::init(build_shape_registry(&docs_dir));

    render::run(&docs_dir);
}

/// Bundled built-in shapes — `res/shapes/*.shape` baked into the exe.
const BUILTIN_SHAPES: &[(&str, &str)] = &[
    ("cloud",    include_str!("../res/shapes/cloud.shape")),
    ("document", include_str!("../res/shapes/document.shape")),
];

fn build_shape_registry(docs_dir: &std::path::Path) -> mermaid::shape_def::ShapeRegistry {
    let mut reg = mermaid::shape_def::ShapeRegistry::new();

    for (name, src) in BUILTIN_SHAPES {
        match mermaid::shape_def::parse(src) {
            Ok(def)  => { reg.insert(def); }
            Err(e)   => eprintln!("built-in shape `{}`: {}", name, e),
        }
    }

    // Per-doc-set overrides — same name as a built-in replaces it.
    let user_shapes = docs_dir.join(".shapes");
    if user_shapes.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&user_shapes) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) != Some("shape") { continue; }
                match std::fs::read_to_string(&p) {
                    Ok(src) => match reg.load_text(&src) {
                        Ok(_)  => {}
                        Err(e) => eprintln!("shape `{}`: {}", p.display(), e),
                    },
                    Err(e) => eprintln!("shape `{}`: {}", p.display(), e),
                }
            }
        }
    }

    reg
}
