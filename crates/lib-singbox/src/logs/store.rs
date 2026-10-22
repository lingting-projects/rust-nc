//! 日志数据的存储。
//!
//! 读取线程把增量解析出的块累积到内部列表，生成不可变快照后整体替换，
//! 读取方直接取 [`Arc`]；因此一次读取不复制日志内容，也不会与读取线程互相阻塞。

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use arc_swap::ArcSwapOption;

use super::block::{Level, LogBlock, LogSnapshot, MAX_TEXT};
use super::stream;

/// 详情读取的单块字节数，边读边汇报进度，也便于调用方尽早中断。
const READ_CHUNK: usize = 64 * 1024;

/// 保留的日志块数量上限。
const CAPACITY: usize = 10000;

/// 保留的日志文本上限，单位字节，与块数上限先到先限。
const CAPACITY_BYTES: usize = 8 * 1024 * 1024;

/// 日志存储。
///
/// 进程内只有一份，由读取线程写入、界面读取。
pub struct LogStore {
    /// 最新快照，读取方拿到的都是共享引用。
    snapshot: ArcSwapOption<LogSnapshot>,
    inner: Mutex<Inner>,
    /// 日志文件注入或撤销时的唤醒信号。
    wake: Condvar,
}

/// 读取线程一轮的起点。
///
/// 读取线程自身不保存状态，每轮都从这里接着上次的位置继续，
/// 因此外部重置游标后线程下一轮自然从头读取。
pub(crate) struct Cursor {
    /// 已读取到的字节位置。
    /// 已读取到的字节位置。
    pub position: u64,
    /// 是否已经开始读取：为假时线程会自行决定起点，只回看文件尾部。
    pub started: bool,
    /// 尚未读完的行片段，下次读取时接在开头继续凑成整行。
    pub pending: Vec<u8>,
    /// 最后一个尚未确定结束的块，续行继续并入其中。
    pub open: Option<LogBlock>,
}

/// 存储内部数据。
struct Inner {
    /// 待读取的日志文件，内核未运行时为空。
    file: Option<PathBuf>,
    /// 读取线程是否已经启动，线程常驻，内核启停只换路径。
    started: bool,
    /// 已解析并确定结束的块，最新的在末尾。
    blocks: VecDeque<LogBlock>,
    /// 块内文本累计占用字节数，用于控制内存上限。
    bytes: usize,
    /// 读取游标与未结束的块。
    cursor: Cursor,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            file: None,
            started: false,
            blocks: VecDeque::new(),
            bytes: 0,
            cursor: Cursor {
                position: 0,
                started: false,
                pending: Vec::new(),
                open: None,
            },
        }
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LogStore {
    /// 创建空存储。
    pub fn new() -> Self {
        Self {
            snapshot: ArcSwapOption::empty(),
            inner: Mutex::new(Inner::default()),
            wake: Condvar::new(),
        }
    }

    /// 当前快照，尚未读到内容时为 `None`。
    pub fn snapshot(&self) -> Option<Arc<LogSnapshot>> {
        self.snapshot.load_full()
    }

    /// 注入日志文件并启动读取线程，传入 `None` 表示内核已停止。
    ///
    /// 线程只在首次注入时启动，之后常驻等待路径。内核每次启动都会重写日志文件，
    /// 因此换路径或重新注入都视为一次全新的读取：游标归零、已有内容丢弃。
    pub(crate) fn subscribe(self: &Arc<Self>, file: Option<PathBuf>) {
        let start = {
            let mut inner = self.lock();
            if inner.file != file {
                inner.file = file;
                inner.reset();
                self.snapshot.store(None);
            }
            let start = !inner.started;
            inner.started = true;
            start
        };

        if start {
            stream::spawn(self.clone());
        }
        self.wake.notify_all();
    }

    /// 等待并取出当前日志文件，路径被撤销时继续等待。
    pub(crate) fn wait_file(&self) -> PathBuf {
        let mut inner = self.lock();
        loop {
            if let Some(file) = inner.file.clone() {
                return file;
            }
            inner = self
                .wake
                .wait(inner)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    /// 当前路径是否仍是 `file`，读取中断后据此决定是否重连。
    pub(crate) fn same_file(&self, file: &PathBuf) -> bool {
        self.lock().file.as_ref() == Some(file)
    }

    /// 取出本轮读取的起点。
    pub(crate) fn cursor(&self) -> Cursor {
        let inner = self.lock();
        Cursor {
            position: inner.cursor.position,
            started: inner.cursor.started,
            pending: inner.cursor.pending.clone(),
            open: inner.cursor.open.clone(),
        }
    }

    /// 读游标位置。
    ///
    /// 线程尚未读入任何内容时为空，此时由读取线程自行决定从何处开始。
    pub(crate) fn position(&self) -> Option<u64> {
        let inner = self.lock();
        match inner.cursor.started {
            true => Some(inner.cursor.position),
            false => None,
        }
    }

    /// 写入本轮结果并生成新快照。
    pub(crate) fn commit(
        &self,
        mut closed: Vec<LogBlock>,
        position: u64,
        pending: Vec<u8>,
        open: Option<LogBlock>,
    ) {
        let mut inner = self.lock();
        if inner.file.is_none() {
            // 本轮读取期间内核已停止，结果作废，避免把旧内容写回已清空的存储
            return;
        }

        for block in closed.drain(..) {
            inner.bytes += block.text.len();
            inner.blocks.push_back(block);
        }
        inner.bytes = inner.bytes.saturating_sub(match &inner.cursor.open {
            Some(block) => block.text.len(),
            None => 0,
        });
        if let Some(block) = &open {
            inner.bytes += block.text.len();
        }
        inner.cursor = Cursor {
            position,
            started: true,
            pending,
            open,
        };
        inner.trim();

        let snapshot = Arc::new(LogSnapshot {
            blocks: inner.blocks.iter().cloned().collect(),
        });
        self.snapshot.store(Some(snapshot));
    }

    /// 丢弃全部内容并把读游标归零。
    ///
    /// 内核启动会重写日志文件，旧块按字节定位会指向新文件的错误位置，
    /// 因此游标、已解析的块与未读完的行片段必须一起丢弃。
    pub(crate) fn reset(&self) {
        let mut inner = self.lock();
        inner.reset();
        self.snapshot.store(None);
    }

    /// 读取一个块的完整内容。
    ///
    /// 块只保留首行的正文，完整内容按字节偏移分块读全，不设任何长度上限：
    /// 详情要展示的就是块的全部原始内容。每读完一块调用一次 `progress`
    /// 汇报已读字节，返回 `false` 表示调用方已经放弃，读取立即停止。
    ///
    /// 读到的正文必须与内存中保存的一致，否则说明内核重写过日志、
    /// 该块的偏移已经失效，此时返回 `None`。
    pub fn read(&self, block: &LogBlock, mut progress: impl FnMut(u64) -> bool) -> Option<String> {
        let file = self.lock().file.clone()?;
        let mut handle = File::open(file).ok()?;
        handle.seek(SeekFrom::Start(block.start)).ok()?;

        // 先按块的真实跨度分配，避免逐块扩容时反复拷贝整段内容
        let mut buffer = Vec::with_capacity(block.bytes as usize);
        let mut reader = (&mut handle).take(block.bytes);
        let mut chunk = vec![0u8; READ_CHUNK];
        let mut read = 0u64;

        loop {
            let size = reader.read(&mut chunk).ok()?;
            if size == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..size]);
            read += size as u64;
            if !progress(read) {
                return None;
            }
        }

        // 块跨度按文件中的原始字节算，读不满说明文件已被截断，偏移失效
        if read != block.bytes {
            return None;
        }

        // 合法 UTF-8 直接复用缓冲区，不做二次拷贝；非法时再退回有损转换
        let content = match String::from_utf8(buffer) {
            Ok(content) => content,
            Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
        };
        // 前缀只可能出现在首行，跳过它再比对内存里保存的正文
        let body = content.get(block.prefix_bytes as usize..)?;
        match body.starts_with(block.text.as_ref()) {
            true => Some(content),
            false => None,
        }
    }

    /// 取内部数据，锁被污染时继续使用其内容。
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}

impl Inner {
    /// 清空读取状态。
    fn reset(&mut self) {
        self.blocks.clear();
        self.bytes = 0;
        self.cursor = Cursor {
            position: 0,
            started: false,
            pending: Vec::new(),
            open: None,
        };
    }

    /// 按块数与字节数两个上限裁剪最旧的块。
    fn trim(&mut self) {
        while self.blocks.len() > CAPACITY || (self.bytes > CAPACITY_BYTES && self.blocks.len() > 1)
        {
            let Some(block) = self.blocks.pop_front() else {
                break;
            };
            self.bytes = self.bytes.saturating_sub(block.text.len());
        }
    }
}

/// 由一行原文新建一个块。
///
/// `end` 是该行在文件中结束的位置（不含行尾换行），块跨度以它为准；
/// 行首的时间与等级前缀单独拆出，正文超过 [`MAX_TEXT`] 时只保留前缀部分，
/// 剩余内容在详情里按字节偏移回读。
pub(crate) fn new_block(start: u64, end: u64, text: &[u8]) -> LogBlock {
    // 先按有损转换取整行：前缀只可能出现在行首，非法 UTF-8 不会影响后半段正文的切分
    let line = String::from_utf8_lossy(text);
    let (level, prefix, time) = Level::scan(&line);
    let time = time.unwrap_or_default();
    let body = &line[prefix..];

    let text_bytes = body.len() as u64;
    let cut = body.len().min(MAX_TEXT);
    // 截断点可能落在多字节字符内部，向前退到字符边界，避免产生非法 UTF-8
    let cut = match std::str::from_utf8(&body.as_bytes()[..cut]) {
        Ok(_) => cut,
        Err(error) => error.valid_up_to(),
    };

    LogBlock {
        start,
        bytes: end.saturating_sub(start),
        level,
        text: Arc::from(&body[..cut]),
        time: Arc::from(time),
        text_bytes,
        prefix_bytes: prefix as u64,
        lines: 1,
    }
}

/// 把一行延续到已有的块上：延长跨度并累计行数，文本仍取首行。
///
/// `end` 与 [`new_block`] 同义，续行一律累计到其中，因此 `bytes`
/// 始终等于块在文件中的真实跨度，详情据此可以一次读全。
pub(crate) fn extend(block: &mut LogBlock, end: u64) {
    block.bytes = end.saturating_sub(block.start);
    block.lines = block.lines.saturating_add(1);
}
