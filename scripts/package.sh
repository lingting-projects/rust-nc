#!/bin/bash

DIR_ROOT=$(cd "$(dirname "$0")" && cd .. && pwd)
DIR_TAR="$DIR_ROOT/target/tar"

NAME="lingting-nc"
VERSION=$(grep "version =" Cargo.toml | head -n 1 | cut -d '"' -f 2)
DESC=$(grep "description =" Cargo.toml | head -n 1 | cut -d '"' -f 2)

if [ -z "$VERSION" ]; then
  echo "获取版本失败!"
  exit 1
fi

echo "配置:"
echo "  名称: $NAME"
echo "  版本: $VERSION"
echo "  描述: $DESC"

_build=false
_ui=false
_tar=false
# 是否编译当前平台特定的包
_special=false
_args=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -b) _build=true; shift 1 ;;
        -i) _ui=true; shift 1 ;;
        -t) _tar=true; shift 1 ;;
        -s) _special=true; shift 1 ;;
        *) _args+=("$1"); shift ;;
    esac
done

if [ $_build = true ]; then
  echo "编译主程序"
  cd "$DIR_ROOT"
  bash scripts/build.sh "${_args[@]}"
  _bc=$?

  if [ $_bc -ne 0 ]; then
    echo "编译异常! code: $c"
    exit 1
  fi
  echo "分发主程序编译结果"
  cd "$DIR_ROOT/target"
  cp -f release/$NAME $DIR_TAR/$NAME
  cp -f release/$NAME.exe $DIR_TAR/$NAME.exe
  cp -rf ../icons $DIR_TAR
fi

if [ $_ui = true ]; then
  echo "编译UI"
  cd "$DIR_ROOT/ui"
  tyarn build:prod
  _uc=$?
  if [ $_uc -ne 0 ]; then
    echo "编译异常! code: $c"
    exit 1
  fi
  echo "分发UI编译结果"
  cp -rf dist "$DIR_TAR/ui"
fi

if [ $_tar = true ]; then
  echo "生成压缩包"
  cd $DIR_TAR
  tar zcvf $NAME.tar.gz icons/ $NAME $NAME.exe $NAME-singbox libsingbox.* ui/
  mv $NAME.tar.gz ../
fi

if [ $_special != true ]; then
  exit 0
fi

if [ "$OS" != "Windows_NT" ]; then
  exit 0
fi

echo "生成msi安装包"
cd $DIR_TAR
cp -f "$DIR_ROOT/assets/wix.wxs" product.wxs
sed -i "s/_name/$NAME/g" product.wxs
sed -i "s/_version/$VERSION/g" product.wxs
sed -i "s/_desc/$DESC/g" product.wxs

wix_args=()

if [ -f "$NAME-singbox" ]; then
  echo "singbox: bin"
  wix_args+=("-d" "BIN=1")
else
  echo "singbox: lib"
  wix_args+=("-d" "LIB=1")
fi

if [ -d ui ]; then
  echo "ui: inner"
  wix_args+=("-d" "UI=1")
fi

echo "编译msi"
# 当前平台格式的安装包
wix build product.wxs \
  -o "$DIR_ROOT/target/$NAME.msi" \
  -ext WixToolset.UI.wixext \
  ${wix_args[@]}
