// rust/build.rs
// 仅构建 bin 模式的内核，产物复制到 target 目录与 target/tar 目录。

use std::env;
use std::fs::{copy, create_dir_all};
use std::path::Path;
use std::process::Command;

use std::sync::LazyLock;

static BIN_NAME: &str = "lingting-nc-singbox";

static LIB_NAME: LazyLock<String> = LazyLock::new(|| {
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

fn main() {
    let target = env::var("TARGET").expect("env TARGET err");
    let dir_out = env::var("OUT_DIR").expect("env OUT_DIR err");
    let dir_manifest = env::var("CARGO_MANIFEST_DIR").expect("env CARGO_MANIFEST_DIR err");
    let path_bin = Path::new(&dir_out).join(BIN_NAME);
    let path_lib = Path::new(&dir_out).join(&*LIB_NAME);

    println!("cargo-bin={}", BIN_NAME);
    println!("cargo-platform={target}");
    println!("cargo-out={dir_out}");
    println!("cargo-manifest={dir_manifest}");

    build_go(&target, &path_bin, Path::new(&dir_manifest));

    // 构建的输出根文件夹
    let dir_build = path_bin.ancestors().nth(4).expect("target dir err");
    println!("cargo-dir={}", dir_build.display());
    let dir_target = dir_build.parent().expect("target dir err");
    println!("cargo-target={}", dir_target.display());
    let dir_tar = dir_target.join("tar");
    if !dir_tar.exists() {
        create_dir_all(&dir_tar).expect("create tar dir err");
    }

    if path_bin.exists() {
        copy(&path_bin, dir_tar.join(BIN_NAME)).expect("copy bin err");
        copy(&path_bin, dir_build.join(BIN_NAME)).expect("copy bin err");
    }
    // 兼容旧版 lib 模式的安装包结构
    if path_lib.exists() {
        copy(&path_lib, dir_tar.join(&*LIB_NAME)).expect("copy lib err");
        copy(&path_lib, dir_build.join(&*LIB_NAME)).expect("copy lib err");
    }

    // 确保重新构建的条件
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=core.go");
    println!("cargo:rerun-if-changed=debug_on.go");
    println!("cargo:rerun-if-changed=debug_off.go");
    println!("cargo:rerun-if-changed=cmd/main.go");
    println!("cargo:rerun-if-changed=go.mod");
    println!("cargo:rerun-if-changed=go.sum");

    build_proto(Path::new(&dir_manifest));
}

/// 由官方 proto 生成 gRPC 客户端代码。
///
/// proto 取自 sing-box 对应版本的 `daemon/started_service.proto` 并保持原样，
/// 升级版本时随 `go.mod` 一起替换。`protoc` 使用 vendored 二进制，无需本机安装。
fn build_proto(manifest_dir: &Path) {
    // gRPC 客户端只在 grpc feature 下编译, 未启用时不必生成代码
    if env::var_os("CARGO_FEATURE_GRPC").is_none() {
        return;
    }

    let proto = manifest_dir.join("proto").join("started_service.proto");
    if !proto.exists() {
        return;
    }

    if let Ok(protoc) = protoc_bin_vendored::protoc_bin_path() {
        // SAFETY: build 脚本单线程执行, 设置环境变量不会与其他线程竞争。
        unsafe { std::env::set_var("PROTOC", protoc) };
    }

    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(&[proto.as_path()], &[manifest_dir.join("proto").as_path()])
        .expect("编译内核 gRPC proto 失败");
    println!("cargo:rerun-if-changed=proto/started_service.proto");
}

/// 内核构建标签。
///
/// - `with_utls` / `with_quic`: TLS 指纹与 QUIC 支持, 节点配置依赖。
/// - `badlinkname`: 允许 `go:linkname` 访问标准库内部符号。sing-box 的 TLS read-wait
///   优化依赖它, 缺失时 `badtls.NewReadWaitConn` 直接返回 `ErrInvalid` 静默降级。
/// - `tfogo_checklinkname0`: 与 `badlinkname` 配套, 需同时传链接器 `-checklinkname=0`。
///
/// 不启用 `with_clash_api`: 内核 API 已改为 gRPC (`services[].type = "api"`)。
static GO_TAGS: &str = "with_utls,with_quic,badlinkname,tfogo_checklinkname0";

/// 调试模式构建标签。
///
/// 仅在 debug 编译时追加: 打开后内核接受 `--debug`, 父进程只要求存活,
/// 供集成测试等非 `lingting-nc` 宿主使用; 正式构建的内核里不存在该分支。
static GO_TAG_DEBUG: &str = "ncdebug";

/// 内核链接器参数, 对齐官方 `release/LDFLAGS`。
///
/// `-X runtime.godebugDefault` 关闭标准库 multipathtcp 并恢复 TLS SHA1 兼容,
/// `-checklinkname=0` 是 `badlinkname` 的前提。
static GO_LDFLAGS: &str =
    "-X runtime.godebugDefault=multipathtcp=0,tlssha1=1 -checklinkname=0 -s -buildid=";

/// 本次构建使用的 Go 构建标签。
///
/// debug 编译追加调试标签, 使内核支持 `--debug`; release 编译不含该标签。
fn go_tags() -> String {
    let debug = env::var("DEBUG").is_ok_and(|value| value == "true");
    match debug {
        true => format!("{GO_TAGS},{GO_TAG_DEBUG}"),
        false => GO_TAGS.to_string(),
    }
}
fn build_go(target: &str, bin_path: &Path, manifest_dir: &Path) {
    let mut cmd = Command::new("go");
    cmd.current_dir(manifest_dir)
        .arg("build")
        .arg("-trimpath")
        .arg(format!("-tags={}", go_tags()))
        .arg(format!("-ldflags={GO_LDFLAGS}"))
        .arg("-o")
        .arg(bin_path)
        .arg("./cmd");

    // 依赖图不含 cgo 包, 显式关闭, 避免构建机需要 C 工具链
    cmd.env("CGO_ENABLED", "0");

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

    let status = cmd.status().expect("Failed to execute go build");
    if !status.success() {
        panic!("Go build failed with status: {}", status);
    }
    println!("Built Go binary: {}", bin_path.display());
}
