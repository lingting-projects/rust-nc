use byte_unit::rust_decimal::prelude::ToPrimitive;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;
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
    pub const units: LazyLock<Vec<DataSizeUnit>> = LazyLock::new(|| {
        vec![
            DataSizeUnit::PB,
            DataSizeUnit::TB,
            DataSizeUnit::GB,
            DataSizeUnit::MB,
            DataSizeUnit::KB,
            DataSizeUnit::Bytes,
        ]
    });

    pub const fn step(self) -> u64 {
        match self {
            DataSizeUnit::Bytes => 1,
            DataSizeUnit::KB => 1024 * DataSizeUnit::Bytes.step(),
            DataSizeUnit::MB => 1024 * DataSizeUnit::KB.step(),
            DataSizeUnit::GB => 1024 * DataSizeUnit::MB.step(),
            DataSizeUnit::TB => 1024 * DataSizeUnit::GB.step(),
            DataSizeUnit::PB => 1024 * DataSizeUnit::TB.step(),
        }
    }

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

    pub fn step_f64(self) -> f64 {
        self.step().to_f64().unwrap()
    }

    pub fn of(&self, value: f64) -> Result<DataSize, ParserError> {
        let i = self.step_f64() * value;
        let bytes = i.to_u64().unwrap();
        let (unit, value) = Self::calculate_bytes(bytes);
        Ok(DataSize { bytes, unit, value })
    }

    pub fn calculate_bytes(value: u64) -> (DataSizeUnit, f64) {
        if value >= DataSizeUnit::KB.step() {
            for unit in Self::units.iter() {
                if value >= unit.step() {
                    let f = value as f64 / unit.step_f64();
                    // 保留两位小数
                    let v = format!("{:.2}", f).parse::<f64>().unwrap();
                    return (*unit, v);
                }
            }
        }

        (DataSizeUnit::Bytes, value.to_f64().unwrap())
    }

    pub fn from_str(unit: &str) -> Result<Self, ParserError> {
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

#[derive(Debug, Clone, Copy)]
pub struct DataSize {
    pub bytes: u64,
    pub unit: DataSizeUnit,
    pub value: f64,
}

impl DataSize {
    pub fn of_bytes(bytes: u64) -> Self {
        let (unit, value) = DataSizeUnit::calculate_bytes(bytes);
        DataSize { bytes, unit, value }
    }

    pub fn parse(source: &str) -> Result<Self, ParserError> {
        let source = source.trim();
        if source.is_empty() {
            return Err(ParserError::InvalidFormat(source.to_string()));
        }

        // 尝试匹配数字和单位模式
        let pattern = regex::Regex::new(r"^(\d+(?:\.\d+)?)\s*([A-Za-z]+)$").unwrap();
        if let Some(captures) = pattern.captures(source) {
            let value_str = captures.get(1).unwrap().as_str();
            let unit_str = captures.get(2).unwrap().as_str();

            let unit = DataSizeUnit::from_str(unit_str)?;

            return match value_str.parse::<f64>() {
                Ok(value) => unit.of(value),
                Err(_) => Err(ParserError::InvalidNumber(value_str.to_string())),
            };
        }

        // 尝试直接解析为字节数
        source
            .parse::<u64>()
            .map(DataSize::of_bytes)
            .map_err(|_| ParserError::InvalidFormat(source.to_string()))
    }

    pub fn display(&self) -> String {
        format!("{:.2} {}", self.value, self.unit.name())
    }
}

impl Display for DataSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = self.display();
        f.write_str(&str)
    }
}

impl PartialEq for DataSize {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for DataSize {}
