#[path = "src/icon.rs"]
mod icon;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/icon.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let path =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("cut-planner.ico");
        std::fs::write(&path, icon::ico()).expect("write generated icon");
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(path.to_str().expect("icon path"));
        resource.set("ProductName", "Cut Planner");
        resource.set("FileDescription", "Cut Planner — sheet cutting plans");
        resource.set(
            "LegalCopyright",
            "Copyright (c) 2026 siarheisinelnikau. MIT License.",
        );
        resource
            .compile()
            .expect("compile Windows application resources");
    }
}
