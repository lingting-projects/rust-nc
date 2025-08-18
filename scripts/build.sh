#!/bin/bash

DIR_ROOT=$(cd "$(dirname "$0")" && cd .. && pwd)

profile="prod"
features=()
clean=false

while [ "$#" -gt 0 ]; do
    case "$1" in
        -d) profile="dev"; shift 1 ;;
        -u) profile="uat"; shift 1 ;;
        -p) profile="prod"; shift 1 ;;
        -c) clean=true; shift 1 ;;
        --f) features+=("$2"); shift 2 ;;
        *) echo "未知参数: $1"; shift  ;;
    esac
done

# 添加环境profile作为feature
features+=("$profile")

# 去重feature列表
features=($(printf "%s\n" "${features[@]}" | sort -u))

# 构建feature参数
feature_args=""
if [ ${#features[@]} -gt 0 ]; then
    feature_args="--features \"$(IFS=, ; echo "${features[*]}")\""
fi

build_cmd="cargo build -p binary-ui --release --no-default-features $feature_args"

cd "$DIR_ROOT"

if [ "$clean" = true ]; then
    echo "cargo clean"
    cargo clean
fi

echo "构建配置:"
echo "  环境: $profile"
echo "  Features: ${features[*]}"
echo "  目标 crate: binary-ui"
echo "  指令 : $build_cmd"

eval $build_cmd