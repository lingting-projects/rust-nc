#### linux

> 目前只发布 Linux 桌面端；编译目标固定为 `x86_64-unknown-linux-gnu`，不接受外部传入。

```shell
# 首次编译前安装系统依赖（fontconfig / freetype / wayland / vulkan 等）
bash scripts/install-dependencies.sh
# 构建桌面端并生成 tar.gz 压缩包
bash scripts/package_linux.sh
```

产物位于 `target/`：

- `lingting-nc-linux-x86_64.tar.gz`：内含 `lingting-nc`、`lingting-nc-singbox` 与 `assets/`


#### cloudflare works

```shell
cd crates/bin-cloudflare
# 手动更新配置
pnpm wrangler secret put share --env prod
```
