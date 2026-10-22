//! 日志块的解析与快照。
//!
//! 内核的一条日志可能占多行：异常堆栈、Go 的 error 链都会把续行直接写进文件，
//! 续行没有时间戳与等级前缀。这里把「首行 + 续行」合成一个块，
//! 首行的时间与等级前缀单独拆出字段，块内只保留正文（受单行上限约束），续行只累计行数。

use std::fmt::{Display, Formatter};
use std::sync::Arc;

use lib_core::data_size::DataSize;

/// 单行文本保留上限。
///
/// 首行超过该长度时只保留前缀，剩余内容在详情里按字节偏移从文件回读。
pub const MAX_TEXT: usize = 1024;

/// 内核日志级别。
///
/// 顺序与 sing-box 一致：靠前的更严重。`Unknown` 表示续行或无法识别的前缀。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Level {
    Panic,
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    /// 没有可识别的等级前缀，通常是上一块的续行。
    #[default]
    Unknown,
}

impl Level {
    /// 按行首前缀解析等级。
    ///
    /// 内核日志的前缀有两种形态：带时间戳时形如 `+0800 2026-02-14 10:00:00 INFO ...`，
    /// 不带时间戳时形如 `INFO ...`。时间戳内不含空格之外的字段，因此只看前四个空格分隔的
    /// token 即可覆盖两种形态，且与行长度无关。
    pub fn parse(line: &str) -> Self {
        Self::scan(line).0
    }

    /// 扫描行首前缀，返回等级、前缀长度与等级之前的 token。
    ///
    /// 前缀长度含等级 token 与紧随其后的一个空格，因此 `&line[len..]` 就是消息正文；
    /// 等级之前的 token 原样拼起来就是时间部分（不带时间戳时为空）。
    pub(crate) fn scan(line: &str) -> (Self, usize, Option<&str>) {
        let mut offset = 0usize;
        for (index, token) in line.split(' ').enumerate() {
            if index == 4 {
                break;
            }
            let Some(level) = Self::of_token(token) else {
                offset += token.len() + 1;
                continue;
            };
            let time = match offset {
                0 => None,
                // 去掉等级前的那一个空格
                offset => Some(&line[..offset - 1]),
            };
            let len = (offset + token.len() + 1).min(line.len());
            return (level, len, time);
        }
        (Self::Unknown, 0, None)
    }

    /// 由单个 token 解析等级，无法识别时返回 `None`。
    fn of_token(token: &str) -> Option<Self> {
        Some(match token {
            "PANIC" => Level::Panic,
            "FATAL" => Level::Fatal,
            "ERROR" => Level::Error,
            "WARN" => Level::Warn,
            "INFO" => Level::Info,
            "DEBUG" => Level::Debug,
            "TRACE" => Level::Trace,
            _ => return None,
        })
    }

    /// 等级文本。
    pub fn label(self) -> &'static str {
        match self {
            Level::Panic => "PANIC",
            Level::Fatal => "FATAL",
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
            Level::Unknown => "",
        }
    }

    /// 是否可用于按等级筛选：`Unknown` 表示续行与无法识别的前缀，不参与筛选。
    pub fn is_known(self) -> bool {
        self != Level::Unknown
    }
}

impl Display for Level {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

/// 一个日志块。
///
/// 大多数块只有一行，异常堆栈等情况会带续行。块只保存首行的正文与前缀里的时间，
/// 续行仅记录行数；需要完整内容时按 [`start`](Self::start) 与
/// [`bytes`](Self::bytes) 从文件回读。
#[derive(Debug, Clone)]
pub struct LogBlock {
    /// 块在日志文件中的起始字节，作为块的唯一标识。
    pub start: u64,
    /// 块在日志文件中占用的总字节数，含续行与行尾换行。
    pub bytes: u64,
    /// 首行去掉时间与等级前缀后的正文，最长 [`MAX_TEXT`] 字节。
    pub text: SharedText,
    /// 首行前缀的时间部分，即等级之前的内容，没有时间戳时为空。
    pub time: SharedText,
    /// 首行去掉前缀后的原始字节数，超过 [`MAX_TEXT`] 时表示正文被截断。
    pub text_bytes: u64,
    /// 首行前缀占用的字节数，回读完整内容时据此跳过前缀。
    pub prefix_bytes: u64,
    /// 块的等级，取自首行前缀。
    pub level: Level,
    /// 块包含的行数，首行之外都是续行。
    pub lines: u32,
}

/// 块内文本。用 `Arc<str>` 保存，筛选结果与详情弹窗共享同一份内容。
pub type SharedText = Arc<str>;

impl LogBlock {
    /// 首行是否因超过上限而被截断。
    pub fn truncated(&self) -> bool {
        self.text_bytes > self.text.len() as u64
    }

    /// 未展示的续行数量。
    pub fn hidden_lines(&self) -> u32 {
        self.lines.saturating_sub(1)
    }

    /// 卡片上额外展示的块大小。
    ///
    /// 详情会把整块完整读出来，块越大读取与展示的成本越高，
    /// 因此这里给出大小让用户自行判断是否值得展开。
    pub fn size(&self) -> String {
        DataSize::of_bytes(self.bytes).display()
    }

    /// 是否有卡片上看不到的内容：有续行，或首行超过保存上限被截断。
    ///
    /// 详情会把整块完整读出来，因此只要有未展示内容就必须可以点击。
    pub fn expandable(&self) -> bool {
        self.hidden_lines() > 0 || self.truncated()
    }

    /// 未展示内容的说明文本，无可展开内容时为空。
    ///
    /// 只描述续行数；块大小另行展示，放在这里会重复。
    pub fn hint(&self) -> String {
        match self.hidden_lines() {
            0 => String::new(),
            lines => format!("还有 {lines} 行"),
        }
    }

    /// 关键字是否命中该块，按首行文本做大小写不敏感的包含匹配。
    pub fn matches(&self, keyword: &str) -> bool {
        if keyword.is_empty() {
            return true;
        }
        self.text.to_lowercase().contains(keyword)
    }
}

/// 某一时刻的日志快照。
///
/// 块按写入顺序保存，最新的在末尾，与文件顺序一致。
#[derive(Debug, Default)]
pub struct LogSnapshot {
    /// 全部日志块，最新的在末尾。
    pub blocks: Vec<LogBlock>,
}

impl LogSnapshot {
    /// 是否没有任何日志。
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// 按块起始字节查找。
    pub fn find(&self, start: u64) -> Option<&LogBlock> {
        self.blocks.iter().find(|block| block.start == start)
    }

    /// 包装为共享引用，供界面直接持有。
    pub fn share(self) -> Arc<Self> {
        Arc::new(self)
    }
}
