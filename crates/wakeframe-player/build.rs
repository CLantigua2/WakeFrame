fn main() {
    let root = std::env::var_os("MPV_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
                .join("..")
                .join("..")
                .join("vendor")
                .join("mpv")
        });
    println!("cargo:rustc-link-search=native={}", root.display());
    println!("cargo:rerun-if-env-changed=MPV_SOURCE");
}
