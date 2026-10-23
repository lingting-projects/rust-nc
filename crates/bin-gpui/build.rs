fn main() {
    #[cfg(windows)]
    {
        let icon = std::path::Path::new("../../assets/icons/256x256.ico");
        println!("cargo:rerun-if-changed={}", icon.display());
        if icon.exists() {
            let _ = embed_resource::compile("app.rc", embed_resource::NONE);
        }

        // 管理员清单不在这里嵌：gpui 依赖已经把自带清单编进静态库带进 exe，
        // 再嵌一份会让链接器报 CVT1100 资源重复。发布产物的提权清单
        // 由 scripts/package_windows.sh 在编译后用 mt.exe 整体写入。
        // 因此开发构建保持 gpui 默认的普通权限，不会每次调试都弹 UAC。
        println!("cargo:rerun-if-changed=app.manifest");
    }
}
