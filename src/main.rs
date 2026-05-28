#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod docs;
mod layout;
mod mermaid;
mod parser;
mod render;
mod rope_buffer;
mod search;
mod theme;

use std::path::PathBuf;
use windows::Win32::UI::HiDpi::*;

fn main() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let launch = parse_launch_args().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

    if !launch.docs_dir.exists() {
        eprintln!("docs directory not found: {}", launch.docs_dir.display());
        std::process::exit(1);
    }

    // Load shape registry — bundled defaults first, then per-doc-set overrides
    // from `<docs>/.shapes/*.shape`. Must happen before any mermaid block is
    // parsed (warm-up thread starts inside `render::run`).
    mermaid::shape_def::init(build_shape_registry(&launch.docs_dir));

    render::run_with_options(
        &launch.docs_dir,
        launch.initial_file.as_deref(),
        launch.test_snap,
        launch.test_snap_scroll_y,
        launch.test_snap_scroll_to_line,
    );
}

struct LaunchArgs {
    docs_dir: PathBuf,
    initial_file: Option<PathBuf>,
    test_snap: bool,
    test_snap_scroll_y: f32,
    test_snap_scroll_to_line: Option<usize>,
}

fn parse_launch_args() -> Result<LaunchArgs, String> {
    let mut args = std::env::args().skip(1);
    match args.next() {
        Some(flag) if flag == "--testsnap" => {
            let Some(file) = args.next() else {
                return Err(testsnap_usage());
            };
            let mut scroll_y = 0.0;
            let mut scroll_to_line = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--scroll" | "--scroll-y" => {
                        let Some(value) = args.next() else {
                            return Err(format!(
                                "{arg} requires a numeric offset\n{}",
                                testsnap_usage()
                            ));
                        };
                        scroll_y = value
                            .parse::<f32>()
                            .map_err(|_| {
                                format!("invalid scroll offset `{value}`\n{}", testsnap_usage())
                            })?
                            .max(0.0);
                    }
                    "--scrollto" | "--scroll-to" => {
                        let Some(value) = args.next() else {
                            return Err(format!(
                                "{arg} requires a source line number\n{}",
                                testsnap_usage()
                            ));
                        };
                        let line = value.parse::<usize>().map_err(|_| {
                            format!("invalid source line `{value}`\n{}", testsnap_usage())
                        })?;
                        scroll_to_line = Some(line.max(1));
                    }
                    other => {
                        return Err(format!(
                            "unknown testsnap option `{other}`\n{}",
                            testsnap_usage()
                        ))
                    }
                }
            }
            let file = absolute_path(PathBuf::from(file));
            if !file.exists() {
                return Err(format!("test snapshot file not found: {}", file.display()));
            }
            if file.is_dir() {
                return Ok(LaunchArgs {
                    docs_dir: file,
                    initial_file: None,
                    test_snap: true,
                    test_snap_scroll_y: scroll_y,
                    test_snap_scroll_to_line: scroll_to_line,
                });
            }
            let docs_dir = file
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            Ok(LaunchArgs {
                docs_dir,
                initial_file: Some(file),
                test_snap: true,
                test_snap_scroll_y: scroll_y,
                test_snap_scroll_to_line: scroll_to_line,
            })
        }
        Some(path) => Ok(LaunchArgs {
            docs_dir: PathBuf::from(path),
            initial_file: None,
            test_snap: false,
            test_snap_scroll_y: 0.0,
            test_snap_scroll_to_line: None,
        }),
        None => {
            let exe = std::env::current_exe().unwrap_or_default();
            Ok(LaunchArgs {
                docs_dir: exe
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join("docs"),
                initial_file: None,
                test_snap: false,
                test_snap_scroll_y: 0.0,
                test_snap_scroll_to_line: None,
            })
        }
    }
}

fn testsnap_usage() -> String {
    "usage: doc-crate.exe --testsnap <markdown-file> [--scroll <offset>] [--scrollto <line>]"
        .to_string()
}

fn absolute_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

/// Bundled built-in shapes — `res/shapes/*.shape` baked into the exe.
const BUILTIN_SHAPES: &[(&str, &str)] = &[
    ("api", include_str!("../res/shapes/api.shape")),
    ("browser", include_str!("../res/shapes/browser.shape")),
    ("cache", include_str!("../res/shapes/cache.shape")),
    ("cloud", include_str!("../res/shapes/cloud.shape")),
    ("database", include_str!("../res/shapes/database.shape")),
    ("db", include_str!("../res/shapes/db.shape")),
    ("disk", include_str!("../res/shapes/disk.shape")),
    ("document", include_str!("../res/shapes/document.shape")),
    ("file", include_str!("../res/shapes/file.shape")),
    ("function", include_str!("../res/shapes/function.shape")),
    ("gateway", include_str!("../res/shapes/gateway.shape")),
    ("internet", include_str!("../res/shapes/internet.shape")),
    ("lock", include_str!("../res/shapes/lock.shape")),
    ("mobile", include_str!("../res/shapes/mobile.shape")),
    ("queue", include_str!("../res/shapes/queue.shape")),
    ("server", include_str!("../res/shapes/server.shape")),
    ("service", include_str!("../res/shapes/service.shape")),
    ("user", include_str!("../res/shapes/user.shape")),
    ("worker", include_str!("../res/shapes/worker.shape")),
];

fn build_shape_registry(docs_dir: &std::path::Path) -> mermaid::shape_def::ShapeRegistry {
    let mut reg = mermaid::shape_def::ShapeRegistry::new();

    for (name, src) in BUILTIN_SHAPES {
        match mermaid::shape_def::parse(src) {
            Ok(def) => {
                reg.insert(def);
            }
            Err(e) => eprintln!("built-in shape `{}`: {}", name, e),
        }
    }

    // Per-doc-set overrides — same name as a built-in replaces it.
    let user_shapes = docs_dir.join(".shapes");
    if user_shapes.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&user_shapes) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) != Some("shape") {
                    continue;
                }
                match std::fs::read_to_string(&p) {
                    Ok(src) => match reg.load_text(&src) {
                        Ok(_) => {}
                        Err(e) => eprintln!("shape `{}`: {}", p.display(), e),
                    },
                    Err(e) => eprintln!("shape `{}`: {}", p.display(), e),
                }
            }
        }
    }

    reg
}
