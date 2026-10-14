//! 数据大小解析与展示。
//!
//! 支持 `1.5TB`、`1024` 等写法, 换算按 1024 进制。

use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Invalid number: {0}")]
    InvalidNumber(String),
    #[error("Invalid unit: {0}")]
    InvalidUnit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSizeUnit {
    Bytes,
    KB,
    MB,
    GB,
    TB,
    PB,
}

impl DataSizeUnit {
    /// 由大到小的全部单位。
    pub const UNITS: [DataSizeUnit; 6] = [
        DataSizeUnit::PB,
        DataSizeUnit::TB,
        DataSizeUnit::GB,
        DataSizeUnit::MB,
        DataSizeUnit::KB,
        DataSizeUnit::Bytes,
    ];

    /// 一个单位包含的字节数。
    pub const fn step(self) -> u64 {
        match self {
            DataSizeUnit::Bytes => 1,
            DataSizeUnit::KB => 1024,
            DataSizeUnit::MB => 1024 * 1024,
            DataSizeUnit::GB => 1024 * 1024 * 1024,
            DataSizeUnit::TB => 1024 * 1024 * 1024 * 1024,
            DataSizeUnit::PB => 1024 * 1024 * 1024 * 1024 * 1024,
        }
    }

    /// 单位名称。
    pub const fn name(self) -> &'static str {
        match self {
            DataSizeUnit::Bytes => "B",
            DataSizeUnit::KB => "KB",
            DataSizeUnit::MB => "MB",
            DataSizeUnit::GB => "GB",
            DataSizeUnit::TB => "TB",
            DataSizeUnit::PB => "PB",
        }
    }

    /// 单位对应的字节数。
    pub fn step_f64(self) -> f64 {
        self.step() as f64
    }

    /// 以当前单位构建大小。
    pub fn of(self, value: f64) -> Result<DataSize, ParserError> {
        let bytes = (self.step_f64() * value) as u64;
        Ok(DataSize::of_bytes(bytes))
    }

    /// 计算最合适的单位与数值, 数值保留两位小数。
    pub fn calculate_bytes(value: u64) -> (DataSizeUnit, f64) {
        if value >= DataSizeUnit::KB.step() {
            for unit in Self::UNITS.iter() {
                if value >= unit.step() {
                    let size = value as f64 / unit.step_f64();
                    let round = format!("{:.2}", size).parse::<f64>().unwrap_or(size);
                    return (*unit, round);
                }
            }
        }

        (DataSizeUnit::Bytes, value as f64)
    }

    /// 由单位名称解析。
    pub fn parse(unit: &str) -> Result<Self, ParserError> {
        match unit.to_lowercase().as_str() {
            "b" | "bytes" => Ok(DataSizeUnit::Bytes),
            "kb" | "kilobytes" => Ok(DataSizeUnit::KB),
            "mb" | "megabytes" => Ok(DataSizeUnit::MB),
            "gb" | "gigabytes" => Ok(DataSizeUnit::GB),
            "tb" | "terabytes" => Ok(DataSizeUnit::TB),
            _ => Err(ParserError::InvalidUnit(unit.to_string())),
        }
    }
}

/// 数据大小。
#[derive(Debug, Clone, Copy)]
pub struct DataSize {
    /// 字节数
    pub bytes: u64,
    pub unit: DataSizeUnit,
    pub value: f64,
}

impl DataSize {
    /// 由字节数构建。
    pub fn of_bytes(bytes: u64) -> Self {
        let (unit, value) = DataSizeUnit::calculate_bytes(bytes);
        Self { bytes, unit, value }
    }

    /// 解析大小文本, 支持 `1.5TB` 与纯字节数。
    pub fn parse(source: &str) -> Result<Self, ParserError> {
        let source = source.trim();
        if source.is_empty() {
            return Err(ParserError::InvalidFormat(source.to_string()));
        }

        if let Some(captures) = PATTERN.captures(source) {
            let value = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
            let unit = captures.get(2).map(|m| m.as_str()).unwrap_or_default();

            return match value.parse::<f64>() {
                Ok(value) => DataSizeUnit::parse(unit)?.of(value),
                Err(_) => Err(ParserError::InvalidNumber(value.to_string())),
            };
        }

        source
            .parse::<u64>()
            .map(DataSize::of_bytes)
            .map_err(|_| ParserError::InvalidFormat(source.to_string()))
    }

    /// 带单位的文本, 数值保留两位小数。
    pub fn display(&self) -> String {
        format!("{:.2} {}", self.value, self.unit.name())
    }
}

impl Display for DataSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display())
    }
}

impl PartialEq for DataSize {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for DataSize {}

static PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+(?:\.\d+)?)\s*([A-Za-z]+)$").expect("大小正则构建失败"));
