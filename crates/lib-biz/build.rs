// 扫描 src 下的 #[timer] 任务, 生成定时任务名称枚举到 OUT_DIR。

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("env CARGO_MANIFEST_DIR err");
    let out = env::var("OUT_DIR").expect("env OUT_DIR err");
    let source = Path::new(&manifest).join("src");
    let target = Path::new(&out).join("timer_name.rs");

    let code = framework_timer::generate_name::generate(&source).expect("生成定时任务名称枚举失败");
    fs::write(&target, code).expect("写入定时任务名称枚举失败");

    println!("cargo:rerun-if-changed=build.rs");
    // 任务增删改名后需要重新生成, 因此监听整个源码目录
    println!("cargo:rerun-if-changed={}", source.display());
}
