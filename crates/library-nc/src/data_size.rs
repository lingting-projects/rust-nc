use byte_unit::rust_decimal::prelude::ToPrimitive;
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

    pub fn step_f64(self) -> f64 {
        self.step().to_f64().unwrap()
    }

    pub fn of(&self, value: f64) -> Result<DataSize, ParserError> {
        let i = self.step_f64() * value;
        Ok(DataSize {
            bytes: i.to_u64().unwrap(),
        })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataSize {
    pub bytes: u64,
}

impl DataSize {
    pub fn of_bytes(bytes: u64) -> Self {
        DataSize { bytes }
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
}
