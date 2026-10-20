# Workspace 架构

本项目为 lingting network control 的 Rust 业务项目。
本文档是当前工作区 Crate 成员、职责边界、依赖方向与公用配置的唯一事实来源；新增、移除、重命名 Crate，或变更职责与依赖方向时，必须同步更新本文档。

## 当前成员

- `bin-cloudflare`
- `bin-gpui`
- `lib-biz`
- `lib-converter`
- `lib-core`
- `lib-db`
- `lib-fetch`
- `lib-pick`
- `lib-singbox`
- `lib-system`
- `service-core`
- `test-singbox`
- `types-core`

## 架构边界

| 位置 | 职责 |
|---|---|
| `bin-gpui` | 桌面端入口；初始化数据层与界面主题、注册路由与侧栏菜单、打开主窗口。承载页面与 GPUI 组件，并持有运行时的数据层，页面经该数据层调用 `service-core` 与 `lib-biz` 暴露的业务方法。 |
| `bin-cloudflare` | Cloudflare Worker 入口；提供订阅转换与 gist、release 代理等无状态 HTTP 接口，不使用数据库。 |
| `lib-biz` | 核心业务流程；编排订阅等领域操作，并承载需要跨请求存活的全局任务调度能力。内核内置规则由本 Crate 提前落盘到本地，内核配置只引用本地规则集。 |
| `lib-converter` | 节点到内核配置的转换；把 `lib-pick` 提取的节点转换为 clash 或 sing-box 配置文本，不涉及网络与持久化。 |
| `lib-fetch` | 远程资源请求与下载；提供请求与响应模型、基于 `wreq`、`reqwest` 的后端实现，以及按 `Range` 分片并行下载的 `probe`、`download`。 |
| `lib-pick` | 订阅内容提取；识别 clash 配置与分享链接并提取节点，同时解析流量与到期信息。 |
| `lib-singbox` | sing-box 内核管理；仅支持随包分发的二进制内核，负责启动、停止、状态查询与规则集格式转换，并在内核运行期间提供基于内核 gRPC API 的节点切换与连接信息查询。 |
| `lib-system` | 操作系统能力；提供字符集、进程与开机自启等平台相关封装。 |
| `lib-db` | SQLite 数据访问基础设施；提供连接与迁移执行、请求级 `DbContext`、事务上下文及通用查询类型。 |
| `lib-core` | 公共基础能力；提供应用目录、日志、雪花 ID、时间、公共值类型与 URL、体积等通用工具；URL 加速前缀由 `constants::FAST_GITHUB_PREFIX` 统一定义。 |
| `service-core` | 领域业务层；每张业务表均有同名 Service 与 crate 内私有 Repository，Repository 仅由同表 Service 调用。跨表流程以 Manager 形式组织在同一 Crate 的 `manager` 模块。 |
| `test-singbox` | 配置生成的集成测试；读取本地订阅源，用 `lib-pick` 提取节点、`lib-converter` 转换配置、`lib-singbox` 调用内核校验，不参与打包。 |
| `types-core` | 领域类型；存放实体、DTO、设置与规则等模型，是其他 Crate 引用领域类型的唯一来源。 |

`crates/library-*` 与 `crates/binary-*` 为历史实现，不属于本工作区成员，不得在其上新增功能；能力按上表迁移到对应成员后逐步淘汰。

## 依赖方向

```text
bin-gpui -> lib-biz / service-core / lib-db / lib-core / lib-singbox / types-core
bin-cloudflare -> lib-converter / lib-fetch / lib-pick / lib-core
lib-biz -> service-core / lib-converter / lib-fetch / lib-pick / lib-db / lib-core / types-core
service-core -> lib-db / lib-system / lib-core / types-core
lib-converter -> lib-pick / lib-core
lib-singbox -> lib-system / lib-core
lib-fetch -> lib-core
lib-pick -> lib-core
lib-db -> lib-core
lib-system -> lib-core
types-core -> lib-core
test-singbox -> lib-converter / lib-pick / lib-singbox
lib-core -> framework-*
```

依赖只能自上而下：入口层依赖业务与领域层，业务与领域层不得反向依赖入口层、界面组件或具体运行环境。

## 公用配置

- 应用目录根为 `live.lingting.nc.rust`，通过 `lib-core::directorys` 取得数据、缓存与安装目录；其中 `config`、`rule`、`inner_rule` 为运行期文件目录，`inner_rule` 存放内核内置规则集，文件名即内置规则 id。
- 数据库为 SQLite，位于应用数据目录下的 `data.sqlite`；迁移脚本在 `migrations/`，应用启动连接时自动执行未应用的迁移。
- 内核配置只引用本地规则集：远程规则集由 `tbl_inner_rule` 登记，`lib-biz` 的 `inner_rule_sync` 负责下载到 `inner_rule` 目录并把 `ready` 置为 1，内核启动前要求全部就绪，避免内核卡在规则集下载而导致 API 服务不监听。
- 所有业务表 ID 由项目统一 ID 生成器产生，时间字段统一为毫秒级时间戳。
- 共享第三方依赖统一声明在根 `Cargo.toml` 的 `workspace.dependencies`，成员 Crate 通过 `.workspace = true` 引用；内部 Crate 使用相对 `path` 依赖。
- sing-box 内核版本由 `lib-singbox` 的 `go.mod` 与 `src/lib.rs` 的 `VERSION` 共同确定，两者必须一致；升级版本时还需用同版本官方 `daemon/started_service.proto` 替换 `lib-singbox/proto/`，供 gRPC 客户端代码生成。
- 桌面端界面位于仓库根的 `ui` 软链接指向的独立仓库，本工作区不包含前端源码。
