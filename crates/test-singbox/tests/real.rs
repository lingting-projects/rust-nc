//! 真实运行测试。
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
//! 任一环节失败即提前结束, 不再继续后面的源。

use test_singbox::{Kernel, Pipeline, probe, source};

/// 真实运行使用的场景: 不启用 tun, 只保留混合入口。
///
/// 内核直接在本机启动, 创建 tun 需要管理员权限并会接管全局流量, 不适合放进测试。
const SCENARIO: &str = "mixed-route-ipv4";

#[tokio::test]
async fn real() {
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

    for item in &sources {
        let kernel = Kernel::new(&item.name)
            .unwrap_or_else(|error| panic!("[{}] 准备内核失败: {error:#}", item.name));
        let pipeline = Pipeline::run(item, &kernel)
            .unwrap_or_else(|error| panic!("[{}] 生成链路失败: {error:#}", item.name));
        let runtime = pipeline
            .runtime(SCENARIO)
            .unwrap_or_else(|error| panic!("[{}] 构建运行配置失败: {error:#}", item.name));

        log::info!(
            "[{}] 节点数: {}, 入口: {}",
            pipeline.name(),
            pipeline.remote().nodes.len(),
            runtime.proxy()
        );

        kernel
            .start(&runtime)
            .await
            .unwrap_or_else(|error| panic!("[{}] 启动内核失败: {error:#}", item.name));
        // 探测失败同样要停止内核, 因此先取结果再停止
        let result = probe::head(runtime.proxy()).await;
        let stopped = kernel.stop();
        result.unwrap_or_else(|error| panic!("[{}] 代理探测失败: {error:#}", item.name));
        stopped.unwrap_or_else(|error| panic!("[{}] 停止内核失败: {error:#}", item.name));

        log::info!("[{}] 真实运行通过", pipeline.name());
    }
}
