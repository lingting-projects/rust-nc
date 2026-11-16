# 开发环境说明

本文档说明 `cargo run` 启动的开发构建与发布版本的行为差异，以及在这种差异下如何正确测试代理流量。

## 开发构建没有管理员权限

`crates/bin-gpui/build.rs` 不把 `requireAdministrator` 清单编进开发产物（避免每次调试弹 UAC），提权清单由打包脚本在编译后单独写入正式产物。
因此 `cargo run` 启动的进程普通权限运行：

- `lib-biz/src/config_file.rs` 的 `tun()` 在未提权时返回 `false`，生成的内核配置里**没有 tun 入站**。
- 系统 DNS 与系统流量都不经过这个内核，它只提供几个本地代理端口。

运行目录与端口（开发构建的默认值，与正式版错开）：

| 项 | 开发构建 | 正式版本 |
| --- | --- | --- |
| 运行目录 | `target/debug/runtime` | 安装目录 |
| 内核面板端口 | 9091 | 9090 |
| 混合代理端口 | 7891 | 7890 |
| 代理入口端口 | 7893 | 7892 |
| fake ip v4 网段 | `172.24.0.0/24` | `172.20.0.0/14` |
| fake ip v6 网段 | `fd93:0d0b:4e8a:2234::/64` | `fd93:0d0b:4e8a:2233::/64` |

内核自身日志在 `target/debug/runtime/singbox/kernel.log`。

## 测代理流量必须使用远程 DNS 协议

socks 与 http 代理对域名解析的位置不同：

- `socks://`（curl 按 SOCKS4 处理）与 `socks5://` 由**客户端本地解析**域名，连接时代理只收到 IP。
- `socks5h://` 与 `http://` 由**代理解析**域名，连接时代理收到的是域名。

开发内核没有接管系统 DNS，并且只认得自己签发的 fake ip。
用本地解析的协议时，客户端拿到的是系统 DNS 的结果；若同机还跑着正式版本，它的 tun 会接管系统 DNS 并返回正式版内核签发的 fake ip，开发内核查不到对应记录，直接报错并断开连接：

```text
ERROR router: missing fakeip record, try enable `experimental.cache_file`
```

所以测试代理流量一律使用远程 DNS 协议：

```shell
# 混合代理（走分流规则）
curl -x socks5h://127.0.0.1:7891 -I https://google.com
curl -x http://127.0.0.1:7891 -I https://google.com

# 代理入口（忽略其余规则，强制走代理出口）
curl -x socks5h://127.0.0.1:7893 -I https://google.com
```

以下写法在开发环境下会失败，不要用来判断功能是否正常：

```shell
# SOCKS4，客户端本地解析，代理只收到 IP
curl -x socks://127.0.0.1:7891 -I https://google.com
```

## 与正式版本同时运行

两边的端口、fake ip 网段都已错开，进程本身可以共存。
但系统 DNS 与默认路由只能由一个实例接管：正式版本的 tun 生效时，它才是系统 DNS 的响应方。
因此开发构建的 fake ip 只在客户端把 DNS 查询送进代理（`socks5h`、`http` 等）时才会用到，不要指望它影响系统级流量。
