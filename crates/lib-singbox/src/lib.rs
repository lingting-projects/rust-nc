//! 以子进程方式驱动 sing-box 内核，仅支持 bin 模式。

use std::path::Path;

use anyhow::Result;

mod bin;

/// 随包分发的 sing-box 版本。
pub const VERSION: &str = "v1.11.9";

/// 内核进程可执行文件名，位于安装目录下。
pub const BIN_NAME: &str = "lingting-nc-singbox";

/// 内核运行状态。
#[derive(Debug, Clone, Default)]
pub struct State {
    /// 是否正在运行
    pub running: bool,
    /// 是否发生异常
    pub error: bool,
    /// 异常原因
    pub reason: Option<String>,
}

/// 内核进程能力。
pub trait SingBox {
    /// 查询当前状态。
    fn state(&mut self) -> Result<State>;
    /// 以 `config_path` 为配置、`work_dir` 为工作目录启动内核。
    fn start(&mut self, config_path: &Path, work_dir: &Path) -> Result<()>;
    /// 停止内核。
    fn stop(&mut self) -> Result<()>;
    /// 将 json 规则集转换为 srs 规则集。
    fn json_to_srs(&mut self, json_path: &Path, srs_path: &Path) -> Result<()>;
}

/// 创建内核进程实例，内核可执行文件位于安装目录。
pub fn create() -> Result<bin::BinSingBox> {
    let directory = lib_core::directorys()?;
    bin::BinSingBox::new(&directory.install)
}
