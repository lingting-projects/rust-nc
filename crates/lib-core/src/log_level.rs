//! 内核日志等级。
//!
//! 取值与 sing-box 的 `log.level` 一一对应，顺序即严重程度：
//! 等级越高（越靠前）输出的日志越少。内核只接受单个等级，
//! 因此这里是单值枚举，界面按单选呈现。

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use specta::Type;

/// 内核日志等级。
///
/// 声明顺序与 sing-box 的 `LogLevel` 一致，靠前的更严重。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Panic,
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// 全部等级，从严重到详细。
    pub const ALL: [LogLevel; 7] = [
        LogLevel::Panic,
        LogLevel::Fatal,
        LogLevel::Error,
        LogLevel::Warn,
        LogLevel::Info,
        LogLevel::Debug,
        LogLevel::Trace,
    ];

    /// 写入内核配置的取值，与 sing-box 的 `log.level` 取值一致。
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Panic => "panic",
            LogLevel::Fatal => "fatal",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }

    /// 界面展示名。
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Panic => "panic",
            LogLevel::Fatal => "fatal",
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

impl Default for LogLevel {
    /// 默认等级：开发构建取 `debug` 便于排查，正式版本取 `info`。
    fn default() -> Self {
        if cfg!(debug_assertions) {
            LogLevel::Debug
        } else {
            LogLevel::Info
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim().to_ascii_lowercase();
        Ok(match value.as_str() {
            "panic" => LogLevel::Panic,
            "fatal" => LogLevel::Fatal,
            "error" => LogLevel::Error,
            "warn" | "warning" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => return Err(()),
        })
    }
}
