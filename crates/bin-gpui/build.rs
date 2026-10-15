//! 把 `assets/icons/256x256.ico` 嵌入可执行文件资源区。
//!
//! Windows 下 GPUI 通过 `LoadImageW(IDI_ICON1)` 读取 exe 资源区作为窗口图标，
//! 因此图标必须先作为资源编译进 exe。

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
