//! sing-box 配置生成测试。
//!
//! 源文件放在 `assets/source`, 每个文件是一个订阅源; 该目录被 git 忽略,
//! 因此本地没有源时只输出警告并跳过。生成链路:
//! `lib-pick` 提取节点 -> `lib-converter` 转换配置 -> 结构校验 -> `lib-singbox` 内核校验。
//! 每个源都跑全部生成场景, 任一环节失败即提前结束, 不再继续后面的源与场景。

use test_singbox::{Kernel, Pipeline, source};

#[test]
fn generate() {
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

        log::info!(
            "[{}] 节点数: {}, 场景数: {}",
            pipeline.name(),
            pipeline.remote().nodes.len(),
            pipeline.configs().len()
        );
    }
}
