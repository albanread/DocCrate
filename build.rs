fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("res/dc.ico");
        res.set_manifest_file("res/doc-crate.manifest");
        res.compile().expect("failed to compile Windows resources");
    }
}
