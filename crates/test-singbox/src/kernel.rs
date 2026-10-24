use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use lib_converter::PlatformConfig;
use lib_singbox::{BIN_NAME, SingBox};

use crate::pipeline::Scenario;
use crate::rules;
use crate::runtime::Runtime;

/// 测试产物根目录名, 位于 `target/{profile}` 下。
const WORK_DIR: &str = "test-singbox";

/// 等待内核 API 就绪的超时。
///
/// 与正式启动一致: 进程创建完不代表可用, 必须探测到内核 API 才算启动成功。
const READY_TIMEOUT: Duration = Duration::from_secs(15);

/// 测试用的 sing-box 内核。
pub struct Kernel {
    inner: SingBox,
    /// 用例的工作目录, 配置与规则集都写在这里
    work: PathBuf,
}

impl Kernel {
    /// 为指定用例准备内核与独立工作目录。
    ///
    /// 工作目录每次重建, 避免上一次的缓存与日志干扰校验结果。
    pub fn new(name: &str) -> Result<Self> {
        let profile = profile_dir()?;
        let work = profile.join(WORK_DIR).join(name);
        if work.exists() {
            fs::remove_dir_all(&work)
                .with_context(|| format!("清理工作目录失败: {}", work.display()))?;
        }
        fs::create_dir_all(&work)
            .with_context(|| format!("创建工作目录失败: {}", work.display()))?;

        let inner = SingBox::new(&profile)?;
        Ok(Self { inner, work })
    }

    /// 场景对应的平台配置, 带本地规则集。
    pub fn config(&self, scenario: Scenario) -> Result<PlatformConfig> {
        rules::config(&self.work, scenario)
    }

    /// 写入场景配置并交给内核校验。
    ///
    /// 每个场景使用独立子目录, 避免上一个场景的缓存影响校验。
    pub fn check(&self, scenario: &str, config: &str) -> Result<()> {
        let (path, dir) = self.write(scenario, config)?;
        self.inner.check(&path, &dir)
    }

    /// 启动内核并等待 API 就绪。
    ///
    /// 返回即代表内核已经可用: 探测到内核 API 才算启动成功。
    /// 停止由 [`stop`](Self::stop) 完成, 内核对象释放时同样会停止进程。
    pub async fn start(&self, runtime: &Runtime) -> Result<()> {
        let (path, dir) = self.write(runtime.scenario(), runtime.body())?;
        self.inner.start(&path, &dir, runtime.api())?;
        self.inner.wait_ready(READY_TIMEOUT).await
    }

    /// 停止内核并等待进程回收。
    pub fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    /// 写入场景配置, 返回配置路径与工作目录。
    fn write(&self, scenario: &str, config: &str) -> Result<(PathBuf, PathBuf)> {
        let dir = self.work.join(scenario);
        fs::create_dir_all(&dir).with_context(|| format!("创建工作目录失败: {}", dir.display()))?;

        let path = dir.join("config.json");
        fs::write(&path, config).with_context(|| format!("写入配置失败: {}", path.display()))?;
        Ok((path, dir))
    }
}

/// 当前测试可执行文件所在的 profile 目录, 即 `target/{profile}`。
///
/// 构建脚本已把内核复制到该目录, 集成测试位于 `deps` 子目录下。
fn profile_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("获取当前可执行文件失败")?;
    let dir = exe.parent().context("未找到当前可执行文件所在目录")?;
    let dir = match dir.file_name().is_some_and(|name| name == "deps") {
        true => dir.parent().context("未找到 target profile 目录")?,
        false => dir,
    };

    let bin = dir.join(BIN_NAME);
    if !bin.exists() {
        bail!("未找到 sing-box 内核: {}", bin.display());
    }

    Ok(dir.to_path_buf())
}
