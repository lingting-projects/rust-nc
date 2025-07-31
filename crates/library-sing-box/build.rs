// rust/build.rs
use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // 检测目标平台
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:target={target}");
    println!("cargo:out={out_dir}");
    println!("cargo:manifest={manifest_dir}");

    // 构建Go库
    build_go_library(&target, &out_dir, &manifest_dir);

    // 告诉Cargo在哪里可以找到库
    println!("cargo:rustc-link-search=native={}", out_dir);
    // 确保重新构建的条件
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=main.go");
    println!("cargo:rerun-if-changed=go.mod");
    println!("cargo:rerun-if-changed=go.sum");
    println!("cargo:rerun-if-changed=Makefile");
}

fn build_go_library(target: &str, out_dir: &str, manifest_dir: &str) {
    let go_dir = Path::new(manifest_dir);

    // 确定输出文件名
    let lib_name = if target.contains("windows") {
        "libsingbox.dll"
    } else if target.contains("apple") {
        "libsingbox.dylib"
    } else {
        "libsingbox.so"
    };

    let out_path = Path::new(out_dir).join(lib_name);

    // 构建命令
    let mut cmd = Command::new("go");
    cmd.current_dir(&go_dir)
        .arg("build")
        .arg("-buildmode=c-shared")
        .arg("-o")
        .arg(&out_path);

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
    println!("cargo:rustc-env=LIB_PATH={}", out_path.display());
    println!("Built Go library: {}", out_path.display());
}
