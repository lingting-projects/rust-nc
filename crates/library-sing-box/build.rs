// rust/build.rs
use std::env;
use std::fs::{copy, create_dir_all, remove_file};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

// 确定输出文件名
static lib_name: LazyLock<String> = LazyLock::new(|| {
    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        "libsingbox.dll"
    } else if target.contains("apple") {
        "libsingbox.dylib"
    } else {
        "libsingbox.so"
    }
    .to_string()
});

static bin_name: &'static str = "lingting-nc-singbox";

fn main() {
    let target = env::var("TARGET").expect("env TARGET err");
    let out_dir = env::var("OUT_DIR").expect("env OUT_DIR err");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("env CARGO_MANIFEST_DIR err");
    let lib_path = Path::new(&out_dir).join(&*lib_name);
    let bin_path = Path::new(&out_dir).join(&*bin_name);

    println!("cargo:rustc-env=SING_BOX_DIR={}", out_dir);
    println!("cargo-lib={}", &*lib_name);
    println!("cargo-bin={}", &*bin_name);
    println!("cargo-platform={target}");
    println!("cargo-out={out_dir}");
    println!("cargo-manifest={manifest_dir}");

    if bin_path.exists() {
        remove_file(&bin_path).expect("failed remove bin");
    }
    if lib_path.exists() {
        remove_file(&lib_path).expect("failed remove lib");
    }
    // 构建Go库
    build_go(&target, &lib_path, &bin_path, Path::new(&manifest_dir));

    let target_dir = lib_path.ancestors().nth(5).expect("target dir err");
    println!("cargo-target={}", target_dir.display());
    let tar_dir = target_dir.join("tar");
    // 分发 lib
    if !tar_dir.exists() {
        create_dir_all(&tar_dir).expect("create tar dir err");
    }
    let tar_lib_path = tar_dir.join(&*lib_name);
    if lib_path.exists() {
        copy(lib_path, tar_lib_path).expect("copy lib err");
    }
    let tar_bin_path = tar_dir.join(&*bin_name);
    if bin_path.exists() {
        copy(bin_path, tar_bin_path).expect("copy bin err");
    }

    // 告诉Cargo在哪里可以找到库
    println!("cargo:rustc-link-search=native={}", out_dir);
    // 确保重新构建的条件
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=main.go");
    println!("cargo:rerun-if-changed=go.mod");
    println!("cargo:rerun-if-changed=go.sum");
    println!("cargo:rerun-if-changed=Makefile");
}

fn build_go(target: &str, lib_path: &Path, bin_path: &Path, manifest_dir: &Path) {
    // 构建命令
    let mut cmd = Command::new("go");
    cmd.current_dir(manifest_dir)
        .arg("build")
        .arg("-v")
        .arg("-trimpath")
        .arg("-tags=with_clash_api")
        .arg("-ldflags=-s -buildid=");

    if cfg!(feature = "bin") {
        cmd.arg("-o").arg(&bin_path).arg("./cmd");
    } else {
        cmd.arg("-buildmode=c-shared")
            .arg("-o")
            .arg(&lib_path)
            .arg("./lib");
    }

    // 启用 cgo
    cmd.env("CGO_ENABLED", "1");

    // 设置目标平台
    if target.contains("windows") {
        cmd.env("GOOS", "windows");
        if target.contains("x86_64") {
            cmd.env("GOARCH", "amd64");
        } else if target.contains("i686") {
            cmd.env("GOARCH", "386");
        }
    } else if target.contains("apple") {
        cmd.env("GOOS", "darwin");
        if target.contains("x86_64") {
            cmd.env("GOARCH", "amd64");
        } else if target.contains("aarch64") {
            cmd.env("GOARCH", "arm64");
        }
    } else if target.contains("linux") {
        cmd.env("GOOS", "linux");
        if target.contains("x86_64") {
            cmd.env("GOARCH", "amd64");
        } else if target.contains("aarch64") {
            cmd.env("GOARCH", "arm64");
        } else if target.contains("arm") {
            cmd.env("GOARCH", "arm");
            if target.contains("v7") {
                cmd.env("GOARM", "7");
            } else if target.contains("v6") {
                cmd.env("GOARM", "6");
            }
        }
    }

    // 执行构建
    let status = cmd.status().expect("Failed to execute go build");
    if !status.success() {
        panic!("Go build failed with status: {}", status);
    }
    println!("Built Go library: {}", lib_path.display());
}
