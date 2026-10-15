fn main() {
    #[cfg(windows)]
    {
        let icon = std::path::Path::new("../../assets/icons/256x256.ico");
        println!("cargo:rerun-if-changed={}", icon.display());
        if icon.exists() {
            let _ = embed_resource::compile("app.rc", embed_resource::NONE);
        }
    }
}
