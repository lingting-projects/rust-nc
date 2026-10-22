//! 连接订阅线程。

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};

use super::store::ConnectionStore;
use crate::SingBoxGrpc;

/// 内核推送增量事件的间隔。
const PUSH_INTERVAL: Duration = Duration::from_secs(1);

/// 订阅中断后的重试间隔。
const RETRY_INTERVAL: Duration = Duration::from_secs(1);

/// 启动订阅线程。
pub(crate) fn spawn(store: Arc<ConnectionStore>) {
    let result = std::thread::Builder::new()
        .name("singbox-connection".to_string())
        .spawn(move || run(store));
    if let Err(error) = result {
        log::error!("[内核] 连接订阅线程启动失败: {error}");
    }
}

/// 订阅线程：取到内核客户端后建立订阅，流断开时按间隔重连。
///
/// 线程常驻，内核启停只是换掉客户端；客户端被撤销时清掉旧数据，
/// 避免把上一轮内核的连接继续展示出来。
fn run(store: Arc<ConnectionStore>) {
    let runtime = match runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            log::error!("[内核] 连接订阅线程启动失败: {error:#}");
            return;
        }
    };

    loop {
        let grpc = store.wait_grpc();
        runtime.block_on(watch(&store, &grpc));
        if !store.same_grpc(&grpc) {
            store.reset();
        }
        std::thread::sleep(RETRY_INTERVAL);
    }
}

/// 建立订阅并把推送交给存储，直到流结束。
async fn watch(store: &ConnectionStore, grpc: &Arc<SingBoxGrpc>) {
    let mut stream = match grpc.connection_stream(PUSH_INTERVAL).await {
        Ok(stream) => stream,
        Err(error) => {
            log::warn!("[内核] 订阅连接失败: {error:#}");
            return;
        }
    };

    loop {
        // 内核只在有流量时才推送, 因此按间隔给等待设上限: 超时就重算一次速率,
        // 空闲连接的速率才会回落到 0, 而不是停在最后一次的瞬时值上。
        match tokio::time::timeout(PUSH_INTERVAL, stream.message()).await {
            Ok(Ok(Some(events))) => store.apply(events),
            Ok(Ok(None)) => {
                log::info!("[内核] 连接订阅已结束");
                return;
            }
            Ok(Err(error)) => {
                log::warn!("[内核] 连接订阅中断: {error:#}");
                return;
            }
            Err(_) => store.refresh(),
        }
    }
}

/// 订阅专用的异步运行时。
///
/// 订阅线程是普通线程，请求需要运行时才能驱动；这里自建一个单线程运行时，
/// 不必让调用方把运行时句柄传进来。
fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("创建连接订阅运行时失败")
}
