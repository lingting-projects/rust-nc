fn main() {
    #[cfg(windows)]
    {
        let icon = std::path::Path::new("../../assets/icons/256x256.ico");
        println!("cargo:rerun-if-changed={}", icon.display());
        if icon.exists() {
            let _ = embed_resource::compile("app.rc", embed_resource::NONE);
        }

        // 仅 release 内嵌管理员清单：正式版本双击与开机自启都直接以管理员权限运行，
        // 开发构建保持普通权限，避免每次调试都弹 UAC。
        let profile = std::env::var("PROFILE").unwrap_or_default();
        if profile == "release" {
            for path in ["app.manifest", "app.manifest.rc"] {
                println!("cargo:rerun-if-changed={path}");
            }
            embed_resource::compile("app.manifest.rc", embed_resource::NONE)
                .manifest_optional()
                .unwrap();
        }
    }
}
