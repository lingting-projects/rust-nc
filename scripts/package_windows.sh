#!/bin/bash
#
# 编译并打包 Windows 桌面端。
#
# 用法:
#   bash scripts/package_windows.sh
#
# 依赖:
#   - Rust 与 Go 已在 PATH 中, 内核版本以 lib-singbox/go.mod 为准。
#   - wix 6 已在 PATH 中, 用于生成 msi 安装包。
#
# 编译目标固定在脚本内, 不接受外部传入。
#
# 产物: target/lingting-nc.zip 与 target/lingting-nc.msi

set -euo pipefail

TARGET="x86_64-pc-windows-msvc"

DIR_ROOT=$(cd "$(dirname "$0")" && cd .. && pwd)
DIR_RELEASE="$DIR_ROOT/target/$TARGET/release"
DIR_STAGE="$DIR_ROOT/target/windows"
NAME="lingting-nc"
SINGBOX_NAME="$NAME-singbox"

VERSION=$(grep -m1 '^version = ' "$DIR_ROOT/Cargo.toml" | cut -d '"' -f 2)
DESC=$(grep -m1 '^description = ' "$DIR_ROOT/Cargo.toml" | cut -d '"' -f 2)

if [ -z "$VERSION" ]; then
  echo "获取版本失败!"
  exit 1
fi

echo "配置:"
echo "  名称: $NAME"
echo "  版本: $VERSION"
echo "  描述: $DESC"
echo "  目标: $TARGET"

# 查找 mt.exe: 优先 PATH, 否则从 Windows Kits 的安装目录里取最新版本。
# 只用于给发布产物写入管理员清单, 属于 Windows SDK 自带工具。
find_mt() {
  if command -v mt.exe >/dev/null 2>&1; then
    command -v mt.exe
    return 0
  fi
  local kits="/c/Program Files (x86)/Windows Kits/10/bin"
  [ -d "$kits" ] || return 1
  local path
  path=$(ls -d "$kits"/*/x64/mt.exe 2>/dev/null | sort -V | tail -1)
  [ -n "$path" ] || return 1
  echo "$path"
}

echo "编译主程序"
cd "$DIR_ROOT"
cargo build -p bin-gpui --release --target "$TARGET"

# 内核二进制由 lib-singbox 的 build.rs 编译后复制到 release 目录, 缺失说明内核构建没跑
if [ ! -f "$DIR_RELEASE/$SINGBOX_NAME" ]; then
  echo "未找到内核二进制: $DIR_RELEASE/$SINGBOX_NAME"
  exit 1
fi

# 编译产物里的清单来自 gpui 依赖(asInvoker), 发布版需要管理员权限,
# 用 mt.exe 把 app.manifest 整体写进 exe 的资源区。
# 不放在 build.rs 里做: gpui 的清单是经静态库带入的, 再嵌一份会报 CVT1100 资源重复。
echo "写入管理员清单"
MT=$(find_mt) || {
  echo "未找到 mt.exe, 请安装 Windows SDK 或把 mt.exe 加入 PATH"
  exit 1
}
"$MT" -nologo -manifest "$DIR_ROOT/crates/bin-gpui/app.manifest" \
  "-outputresource:$(cd "$DIR_RELEASE" && pwd -W)/$NAME.exe;1"

echo "准备打包目录"
rm -rf "$DIR_STAGE"
mkdir -p "$DIR_STAGE/assets" "$DIR_STAGE/icons"
cp -f "$DIR_RELEASE/$NAME.exe" "$DIR_STAGE/$NAME.exe"
cp -f "$DIR_RELEASE/$SINGBOX_NAME" "$DIR_STAGE/$SINGBOX_NAME"

# 运行期资源: 程序按可执行文件同级的 assets/ 读取图标, 与仓库目录结构保持一致
cp -rf "$DIR_ROOT/assets/icons" "$DIR_STAGE/assets/icons"

# 安装包的产品图标, 单独放一份供 wix 的 Icon 引用
cp -f "$DIR_ROOT/assets/icons/256x256.ico" "$DIR_STAGE/icons/256x256.ico"

echo "生成 zip 免安装包"
cd "$DIR_STAGE"
rm -f "$DIR_ROOT/target/$NAME.zip"
# Git Bash 不自带 zip, 依次尝试 7z、zip, 最后回退到 PowerShell
if command -v 7z >/dev/null 2>&1; then
  7z a -tzip -bso0 -bsp0 "$DIR_ROOT/target/$NAME.zip" "$NAME.exe" "$SINGBOX_NAME" assets
elif command -v zip >/dev/null 2>&1; then
  zip -q "$DIR_ROOT/target/$NAME.zip" "$NAME.exe" "$SINGBOX_NAME" assets
else
  powershell.exe -NoProfile -NonInteractive -Command \
    "Compress-Archive -Force -Path '$NAME.exe','$SINGBOX_NAME','assets' -DestinationPath '$DIR_ROOT/target/$NAME.zip'"
fi

echo "生成 msi 安装包"
# wix 的 Source 是相对编译指令执行位置的路径, 所以模板复制到打包目录再编译
cp -f "$DIR_ROOT/assets/wix.wxs" product.wxs
sed -i "s/_name/$NAME/g" product.wxs
sed -i "s/_version/$VERSION/g" product.wxs
sed -i "s/_desc/$DESC/g" product.wxs

# 模板用到 util:CloseApplication, 需要 WixToolset.Util.wixext 扩展;
# 缓存里没有时先添加, 已缓存时 wix 会直接复用, 重复执行不会报错。
wix extension add -g WixToolset.Util.wixext/6.0.1

wix build product.wxs \
  -d IconDesktop=0 \
  -d BIN=1 \
  -ext WixToolset.Util.wixext \
  -o "$DIR_ROOT/target/$NAME.msi"

echo "产物: target/$NAME.zip target/$NAME.msi"
