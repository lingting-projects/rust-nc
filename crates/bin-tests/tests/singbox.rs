//! sing-box 真实运行测试。
//!
//! 源文件放在 `assets/source`, 每个文件是一个订阅源; 该目录被 git 忽略,
//! 因此本地没有源时只输出警告并跳过。每个源依次:
//!
//! 1. 走生成链路产出并校验配置;
//! 2. 把配置改成可运行形态(随机端口 + 默认出口指向节点选择组);
//! 3. 启动内核并等待 API 就绪;
//! 4. 走入口代理用 head 请求访问作者主页;
//! 5. 停止内核。
//!
//! 各源相互独立, 并发执行: 每个源使用独立的内核实例与工作目录, 端口也各自随机,
//! 因此互不干扰; 单个源失败不影响其余源, 最后统一汇总失败项。

use std::time::Duration;

use anyhow::bail;
use bin_tests::singbox::{Kernel, Pipeline, probe};
use bin_tests::source;
use futures_util::future::join_all;

/// 真实运行使用的场景: 不启用 tun, 只保留混合入口。
///
/// 内核直接在本机启动, 创建 tun 需要管理员权限并会接管全局流量, 不适合放进测试。
const SCENARIO: &str = "mixed-route-ipv4";

/// 单个源从生成到探测完成的超时。
///
/// 内核启动与代理探测都要等网络, 不给上限会让测试卡住。
const TIMEOUT: Duration = Duration::from_secs(180);

#[tokio::test(flavor = "multi_thread")]
async fn singbox() -> anyhow::Result<()> {
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
        return Ok(());
    }

    let tasks = sources.iter().map(run);
    let mut failures = Vec::new();
    for (name, result) in join_all(tasks).await {
        match result {
            Ok(()) => log::info!("[{name}] 真实运行通过"),
            Err(error) => {
                log::error!("[{name}] 真实运行失败: {error:#}");
                failures.push(format!("[{name}]"));
            }
        }
    }

    if !failures.is_empty() {
        bail!("sing-box 运行失败: {}", failures.join(", "));
    }

    log::info!("测试通过");
    Ok(())
}

/// 单个源的真实运行流程, 超时按失败处理。
async fn run(item: &source::Source) -> (String, anyhow::Result<()>) {
    let name = item.name.clone();
    let result = tokio::time::timeout(TIMEOUT, execute(item))
        .await
        .unwrap_or_else(|_| bail!("超过 {}s 未完成", TIMEOUT.as_secs()));

    (name, result)
}

/// 走完生成、启动、探测与停止。
async fn execute(item: &source::Source) -> anyhow::Result<()> {
    let kernel = Kernel::new(&item.name)?;
    let pipeline = Pipeline::run(item, &kernel)?;
    let runtime = pipeline.runtime(SCENARIO)?;

    log::info!(
        "[{}] 节点数: {}, 入口: {}",
        pipeline.name(),
        pipeline.remote().nodes.len(),
        runtime.proxy()
    );

    kernel.start(&runtime).await?;
    // 探测失败同样要停止内核, 因此先取结果再停止
    let result = probe::head(runtime.proxy()).await;
    let stopped = kernel.stop();
    result?;
    stopped
}
