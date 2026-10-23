use framework_proc_auto::auto_type;
use lib_core::LogLevel;
use lib_core::constants;

/// 内核设置。
#[auto_type(default = false)]
pub struct SettingKernel {
    /// sing-box 版本
    pub sing_box_version: String,
    /// 面板监听地址
    pub ui_listen: String,
    /// 面板监听端口
    pub ui_port: u16,
    /// 混合代理监听地址
    pub mixed_listen: String,
    /// 混合代理端口
    pub mixed_port: u16,
    /// 国内 dns
    pub dns_cn: Vec<String>,
    /// 代理 dns
    pub dns_proxy: Vec<String>,
    /// 内核日志等级，写入内核配置的 `log.level`，改动后需刷新配置并重启内核才生效
    pub log_level: LogLevel,
    /// 启动后是否自动运行上次选中的配置
    pub auto_run: bool,
}

impl Default for SettingKernel {
    fn default() -> Self {
        Self {
            sing_box_version: String::new(),
            ui_listen: constants::KERNEL_UI_LISTEN.to_string(),
            ui_port: constants::KERNEL_UI_PORT,
            mixed_listen: constants::KERNEL_MIXED_LISTEN.to_string(),
            mixed_port: constants::KERNEL_MIXED_PORT,
            dns_cn: to_vec(constants::KERNEL_DNS_CN),
            dns_proxy: to_vec(constants::KERNEL_DNS_PROXY),
            log_level: LogLevel::default(),
            auto_run: true,
        }
    }
}

/// 软件设置。
#[auto_type(default = false)]
pub struct SettingSoftware {
    /// 是否开机自启, 取自系统计划任务状态
    pub startup: bool,
    /// 是否最小化启动
    pub minimize: bool,
    /// 关闭窗口时是否隐藏到托盘
    pub tray: bool,
    /// 当前软件版本, 取自本 crate 的 CARGO_PKG_VERSION
    pub version: String,
    /// github 加速前缀
    pub fast_github: String,
    /// 连通性测试地址
    pub test_url: String,
}

impl Default for SettingSoftware {
    fn default() -> Self {
        Self {
            startup: false,
            minimize: false,
            tray: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            fast_github: constants::FAST_GITHUB_PREFIX.to_string(),
            test_url: constants::TEST_URL.to_string(),
        }
    }
}

/// 运行设置。
#[auto_type]
pub struct SettingRun {
    /// 启动时是否自动运行选中配置
    pub auto: bool,
    /// 选中的配置 id
    pub selected: Option<String>,
}

/// 应用设置聚合。
#[auto_type]
pub struct Setting {
    pub kernel: SettingKernel,
    pub software: SettingSoftware,
    pub run: SettingRun,
}

fn to_vec(source: &[&str]) -> Vec<String> {
    source.iter().map(|item| item.to_string()).collect()
}
