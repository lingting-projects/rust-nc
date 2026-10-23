#### windows

> 编译目标固定为 `x86_64-pc-windows-msvc`，不接受外部传入；msi 依赖 wix 6。

```shell
# 构建桌面端并生成 zip 免安装包与 msi 安装包
bash scripts/package_windows.sh
```

产物位于 `target/`：

- `lingting-nc.zip`：`lingting-nc.exe`、`lingting-nc-singbox` 与 `assets/`
- `lingting-nc.msi`：安装到 `%ProgramFiles%\lingting\lingting-nc`


#### linux

> 编译目标固定为 `x86_64-unknown-linux-gnu`，不接受外部传入。Linux 产物不参与 CI 发布。

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
