#!/bin/bash

DIR_ROOT=$(cd "$(dirname "$0")" && pwd)

cd "$DIR_ROOT"
bash "$DIR_ROOT/build.sh" "$@"
c=$?

if [ $c -ne 0 ]; then
  echo "编译异常! code: $c"
  exit 1
fi

cd target
cp release/lingting-nc.exe tar/lingting-nc.exe
cp -rf ../icons tar

# 当前平台的 tar.gz 包
cd tar
tar zcvf lingting-nc.tar.gz icons/ lingting-nc.exe lingting-nc-singbox libsingbox.*
mv lingting-nc.tar.gz ../

# 当前平台格式的安装包
