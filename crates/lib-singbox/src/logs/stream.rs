//! 日志读取线程。
//!
//! 按固定间隔检查日志文件的增长，只读取新增字节；每读到一整行就按前缀判断
//! 它是新块的开始还是上一块的续行，凑出完整块后交给存储。
//! 文件比读游标短说明内核重写过日志，此时整体丢弃并从头读。

use std::fs::{File, metadata};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::block::{Level, LogBlock};
use super::store::{self, LogStore};

/// 检查日志增长的间隔。
const INTERVAL: Duration = Duration::from_millis(300);

/// 单轮读取的字节上限，超出部分留到下一轮，避免一次性读入大量积压内容。
const BATCH: u64 = 1024 * 1024;

/// 首次读取时最多回看的字节数，只取尾部内容，不必读入整个历史文件。
const TAIL: u64 = 256 * 1024;

/// 启动读取线程。
pub(crate) fn spawn(store: Arc<LogStore>) {
    let result = thread::Builder::new()
        .name("singbox-log".to_string())
        .spawn(move || run(store));
    if let Err(error) = result {
        log::error!("[内核] 日志读取线程启动失败: {error}");
    }
}

/// 读取线程：取到日志文件后持续读取，内核停止时回到等待。
fn run(store: Arc<LogStore>) {
    loop {
        let path = store.wait_file();
        while store.same_file(&path) {
            match poll(&store, &path) {
                Ok(()) => thread::sleep(INTERVAL),
                Err(error) => {
                    // 文件暂不可读（内核正在创建）时按间隔重试，不退出线程
                    log::debug!("[内核] 读取日志失败: {error}");
                    thread::sleep(INTERVAL);
                }
            }
        }
    }
}

/// 读取一轮新增内容。
fn poll(store: &LogStore, path: &Path) -> std::io::Result<()> {
    let size = match metadata(path) {
        Ok(meta) => meta.len(),
        // 内核刚启动时文件尚未创建，等下一轮
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    // 文件比游标短，说明内核重写过日志，已有块按字节定位全部失效
    if let Some(position) = store.position()
        && size < position
    {
        store.reset();
    }

    let cursor = store.cursor();
    // 首次读取只回看尾部；之后从上次读到的位置继续
    let start = match cursor.started {
        false => size.saturating_sub(TAIL),
        true => cursor.position,
    };
    if start >= size {
        return Ok(());
    }

    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = vec![0u8; (size - start).min(BATCH) as usize];
    let read = file.read(&mut buffer)?;
    buffer.truncate(read);
    if buffer.is_empty() {
        return Ok(());
    }

    let mut offset = start;
    if !cursor.started || start != cursor.position {
        // 从文件中段开始，开头那半行无法确定起点，丢弃到第一个换行之后
        match buffer.iter().position(|byte| *byte == b'\n') {
            Some(index) => {
                buffer.drain(..=index);
                offset += index as u64 + 1;
            }
            None => return Ok(()),
        }
    }

    // 上一轮未凑齐的行片段从 `offset - 片段长度` 处开始
    let base = offset.saturating_sub(cursor.pending.len() as u64);
    let (closed, pending, open) = parse(cursor.pending, cursor.open, base, &buffer);
    let next = offset + buffer.len() as u64;
    store.commit(closed, next, pending, open);
    Ok(())
}

/// 把字节流切成块。
///
/// `pending` 是上一轮末尾尚未凑齐的整行片段，`open` 是上一轮最后一个块：
/// 它的续行可能还没写完，因此先并入，确定结束后再移出。
/// `base` 是 `pending` 首字节在文件中的偏移。
fn parse(
    pending: Vec<u8>,
    open: Option<LogBlock>,
    base: u64,
    buffer: &[u8],
) -> (Vec<LogBlock>, Vec<u8>, Option<LogBlock>) {
    let mut data = pending;
    data.extend_from_slice(buffer);

    let mut closed = Vec::new();
    let mut open = open;
    let mut start = 0usize;
    let mut cursor = 0usize;

    while cursor < data.len() {
        if data[cursor] != b'\n' {
            cursor += 1;
            continue;
        }

        // 去掉行尾的回车，Windows 上内核也可能写出 CRLF
        let mut line = &data[start..cursor];
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        // 块跨度按含换行的原始字节算，详情回读时才能读到完整内容
        apply(
            &mut closed,
            &mut open,
            base + start as u64,
            base + cursor as u64 + 1,
            line,
        );

        cursor += 1;
        start = cursor;
    }

    (closed, data[start..].to_vec(), open)
}

/// 按前缀把一行并入块列表：有等级前缀或空行开新块，其余作为续行。
///
/// `start` 是该行首字节在文件中的位置，`end` 是行尾换行之后的位置，
/// 块跨度据此逐行延长，因此详情回读时能取到整块的原始内容。
fn apply(
    closed: &mut Vec<LogBlock>,
    open: &mut Option<LogBlock>,
    start: u64,
    end: u64,
    line: &[u8],
) {
    let level = match std::str::from_utf8(line) {
        Ok(text) => Level::parse(text),
        // 含非法 UTF-8 的行不可能有等级前缀，按续行处理
        Err(_) => Level::Unknown,
    };

    if level.is_known() || line.is_empty() {
        if let Some(block) = open.take() {
            closed.push(block);
        }
        *open = Some(store::new_block(start, end, line));
        return;
    }

    match open {
        // 续行并入当前块，只有跨度与行数增长，文本仍是首行
        Some(block) => store::extend(block, end),
        // 没有首行的孤立续行，按独立块处理
        None => *open = Some(store::new_block(start, end, line)),
    }
}
