#!/bin/bash

bash ./build.sh "$@"

cp target/release/lingting-nc.exe target/tar/lingting-nc.exe
cp -rf icons target/tar

# 发布当前平台的 tar.gz 包
# 发布当前平台格式的安装包