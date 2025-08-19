// rust/build.rs
use std::env;
use std::fs::{copy, create_dir_all, remove_file};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

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
    let dir_out = env::var("OUT_DIR").expect("env OUT_DIR err");
    let dir_manifest = env::var("CARGO_MANIFEST_DIR").expect("env CARGO_MANIFEST_DIR err");
    let path_lib = Path::new(&dir_out).join(&*lib_name);
    let path_bin = Path::new(&dir_out).join(&*bin_name);

    println!("cargo-lib={}", &*lib_name);
    println!("cargo-bin={}", &*bin_name);
    println!("cargo-platform={target}");
    println!("cargo-out={dir_out}");
    println!("cargo-manifest={dir_manifest}");

    // 构建Go库
    build_go(&target, &path_lib, &path_bin, Path::new(&dir_manifest));

    // 构建的输出根文件夹
    let dir_build = path_lib.ancestors().nth(4).expect("target dir err");
    println!("cargo-dir={}", dir_build.display());
    let dir_target = dir_build.parent().expect("target dir err");
    println!("cargo-target={}", dir_target.display());
    let dir_tar = dir_target.join("tar");
    // 分发 lib
    if !dir_tar.exists() {
        create_dir_all(&dir_tar).expect("create tar dir err");
    }
    if path_lib.exists() {
        copy(&path_lib, dir_tar.join(&*lib_name)).expect("copy lib err");
        copy(&path_lib, dir_build.join(&*lib_name)).expect("copy lib err");
    }
    if path_bin.exists() {
        copy(&path_bin, dir_tar.join(&*bin_name)).expect("copy bin err");
        copy(&path_bin, dir_build.join(&*bin_name)).expect("copy bin err");
    }

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
