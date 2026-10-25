#!/bin/bash

DIR_ROOT=$(cd "$(dirname "$0")" && cd .. && pwd)
VERSION=$(grep "version =" Cargo.toml | head -n 1 | cut -d '"' -f 2)

if [ -z "$VERSION" ]; then
  echo "获取版本失败!"
  exit 1
fi

echo "配置:"
echo "  版本: $VERSION"

_publish=false

while [ "$#" -gt 0 ]; do
    case "$1" in
        -p) _publish=true; shift 1 ;;
        -d) _publish=false; shift 1 ;;
        *) shift 1;;
    esac
done

# 工作目录到 cloudflare
cd crates/bin-cloudflare

# 不知道为什么 用户令牌一直提示无权限, 改成账号令牌
export CLOUDFLARE_API_TOKEN="$CLOUDFLARE_ACCOUNT_TOKEN"

if [ $_publish = true ]; then
    # 替换配置
    sed -i "s/0.0.0/$VERSION/g" package.json
    npm install
    # 发布
    npm run deploy
    exit 0
fi

pnpm wrangler dev