//! `bin-cloudflare` 接口测试。
//!
//! 本地用 `wrangler dev` 拉起 worker, 校验分享与订阅转换两类接口:
//!
//! 1. 分享: `/s/{key}` 按环境变量 `share` 返回 307 重定向, 参数不匹配或分享键
//!    不存在时返回 403;
//! 2. 转换: 每个订阅源分别请求 `/clash` 与 `/sing-box`, 校验响应头与配置结构。
//!
//! 订阅源由本地假订阅服务提供, 内容取自 `assets/source`, 因此不依赖外网;
//! 该目录被 git 忽略, 本地没有源时只输出警告并跳过。分享与转换共用同一个 worker,
//! 避免重复编译 wasm 产物; 各源的转换并发执行, 单个源失败不影响其余源。

use std::time::Duration;

use anyhow::{anyhow, bail};
use bin_tests::cloudflare::{Platform, Worker, share};
use bin_tests::server::Server;
use bin_tests::source;
use futures_util::future::join_all;

/// 单个源单次转换的超时。
///
/// 转换需要 worker 拉取订阅并生成配置, 源较大时耗时明显, 不给上限会卡住测试。
const TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::test(flavor = "multi_thread")]
async fn cloudflare() {
    // 测试进程只初始化一次日志, 已初始化时忽略
    let _ = simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init();

    let sources = source::load_all().expect("读取订阅源失败");
    if sources.is_empty() {
        log::warn!(
            "未找到订阅源, 跳过测试! 请把源文件放入 {}",
            source::SOURCE_DIR
        );
        return;
    }

    let server = Server::start().expect("启动假订阅服务失败");
    log::info!("假订阅服务: {}", server.base());

    let config = share::config();
    let mut worker = Worker::start(&[(share::VARIABLE, config.as_str())])
        .await
        .unwrap_or_else(|error| panic!("启动 worker 失败: {error:#}"));

    // 分享与转换互不影响, 两边都跑完再停止 worker
    let sharing = sharing(worker.base());
    let converting = converting(worker.base(), &server, &sources).await;
    let stopped = worker.stop();

    sharing.unwrap_or_else(|error| panic!("分享接口校验失败: {error:#}"));
    converting.unwrap_or_else(|error| panic!("转换接口校验失败: {error:#}"));
    stopped.unwrap_or_else(|error| panic!("停止 worker 失败: {error:#}"));

    log::info!("测试通过");
}

/// 分享接口校验。
fn sharing(worker: &str) -> anyhow::Result<()> {
    let plain = format!("/s/{}?p=1", share::KEY_PLAIN);
    let plain_deny = format!("/s/{}", share::KEY_PLAIN);
    let secure = format!("/s/{}", share::KEY_SECURE);
    let secure_ok = format!("{secure}?p={}", share::SECURE_P);

    let cases = [
        (plain.as_str(), Some(share::PLAIN_URL)),
        (secure_ok.as_str(), Some(share::SECURE_URL)),
        // 缺少 p 参数时一律拒绝, 明文地址也不例外
        (plain_deny.as_str(), None),
        // 带参数的地址参数不匹配时必须拒绝
        (secure.as_str(), None),
        ("/s/secure?p=bad", None),
        // 分享键不存在
        ("/s/not-exists?p=1", None),
    ];

    for (path, target) in cases {
        let response = share::get(worker, path)?;
        match target {
            Some(target) => share::expect_redirect(&response, target)?,
            None => share::expect_deny(&response)?,
        }
        log::info!("[分享] {path} 校验通过");
    }

    Ok(())
}

/// 转换接口校验: 每个源在每个平台各转换一次, 全部并发执行。
///
/// 各源相互独立, 单个源失败不影响其余源, 最后统一汇总失败项。
async fn converting(
    worker: &str,
    server: &Server,
    sources: &[source::Source],
) -> anyhow::Result<()> {
    let tasks = sources.iter().flat_map(|item| {
        Platform::ALL.into_iter().map(move |platform| {
            let url = server.url(&item.name);
            let name = item.name.clone();
            async move {
                let result = convert(worker, &url, platform).await;
                (name, platform, result)
            }
        })
    });

    let mut failures = Vec::new();
    for (name, platform, result) in join_all(tasks).await {
        match result {
            Ok(()) => log::info!("[转换] [{name}] {} 校验通过", platform.path()),
            Err(error) => {
                log::error!("[转换] [{name}] {} 校验失败: {error:#}", platform.path());
                failures.push(format!("[{name}] {}", platform.path()));
            }
        }
    }

    match failures.is_empty() {
        true => Ok(()),
        false => bail!("转换失败: {}", failures.join(", ")),
    }
}

/// 校验单个源在指定平台的转换结果, 超时按失败处理。
async fn convert(worker: &str, url: &str, platform: Platform) -> anyhow::Result<()> {
    let request = platform.request(worker, url);
    let response = tokio::time::timeout(TIMEOUT, request)
        .await
        .map_err(|_| anyhow!("{} 超过 {}s 未返回", platform.path(), TIMEOUT.as_secs()))??;

    platform.verify(&response)
}
