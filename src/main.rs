#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod docs;
mod layout;
mod parser;
mod render;
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

    render::run(&docs_dir);
}
