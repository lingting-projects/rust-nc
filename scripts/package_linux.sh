#!/bin/bash
#
# 编译并打包 Linux 桌面端。
#
# 用法:
#   bash scripts/package_linux.sh
#
# 编译目标固定在脚本内, 不接受外部传入: GPUI 依赖 fontconfig / freetype /
# wayland / vulkan 等系统库, 走 musl 静态目标需要把这些库逐个从源码编译, 不现实,
# 因此这里固定为动态链接的 x86_64-unknown-linux-gnu。

set -euo pipefail

TARGET="x86_64-unknown-linux-gnu"
ARCH="${TARGET%%-*}"

DIR_ROOT=$(cd "$(dirname "$0")" && cd .. && pwd)
DIR_RELEASE="$DIR_ROOT/target/$TARGET/release"
DIR_STAGE="$DIR_ROOT/target/linux"
NAME="lingting-nc"
SINGBOX_NAME="$NAME-singbox"
ARCHIVE="$NAME-linux-$ARCH.tar.gz"

if [ "$(uname -s)" != "Linux" ]; then
  echo "仅支持在 Linux 上打包!"
  exit 1
fi

VERSION=$(grep -m1 '^version = ' "$DIR_ROOT/Cargo.toml" | cut -d '"' -f 2)

if [ -z "$VERSION" ]; then
  echo "获取版本失败!"
  exit 1
fi

echo "配置:"
echo "  名称: $NAME"
echo "  版本: $VERSION"
echo "  目标: $TARGET"

echo "编译主程序"
cd "$DIR_ROOT"
cargo build -p bin-gpui --release --target "$TARGET"

# 内核二进制由 lib-singbox 的 build.rs 编译后复制到 release 目录, 缺失说明内核构建没跑
if [ ! -f "$DIR_RELEASE/$SINGBOX_NAME" ]; then
  echo "未找到内核二进制: $DIR_RELEASE/$SINGBOX_NAME"
  exit 1
fi

echo "准备打包目录"
rm -rf "$DIR_STAGE"
mkdir -p "$DIR_STAGE/assets"
cp -f "$DIR_RELEASE/$NAME" "$DIR_STAGE/$NAME"
cp -f "$DIR_RELEASE/$SINGBOX_NAME" "$DIR_STAGE/$SINGBOX_NAME"

# 运行期资源: 程序按可执行文件同级的 assets/ 读取图标, 与仓库目录结构保持一致
cp -rf "$DIR_ROOT/assets/icons" "$DIR_STAGE/assets/icons"

echo "生成压缩包"
cd "$DIR_STAGE"
rm -f "$DIR_ROOT/target/$ARCHIVE"
tar czf "$DIR_ROOT/target/$ARCHIVE" .
tar czf "$DIR_ROOT/target/$ARCHIVE" "$NAME" "$SINGBOX_NAME" assets
echo "产物: target/$ARCHIVE"
