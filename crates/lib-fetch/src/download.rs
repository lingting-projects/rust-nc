//! 远程文件下载。
//!
//! 先用 HEAD 探测大小与是否支持分段：支持时按线程数切成若干块并行下载，
//! 每块写入目标文件的对应偏移；不支持分段或文件过小时退回单线程顺序下载。

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use bytes::Bytes;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncSeekExt as _, AsyncWriteExt as _};

use crate::backend;
use crate::client::{FetchClient, with_ua};
use crate::request::FetchRequest;

/// 分片下载的最小文件大小。
///
/// 分片数超过文件大小时每块只剩几百字节，请求开销会盖过并行收益，
/// 因此小于这个值直接单线程下载。
const MIN_CHUNK: u64 = 1024 * 1024;

/// 默认并发分片数。
pub const DEFAULT_THREADS: usize = 12;

/// 远端文件探测结果。
#[derive(Clone, Debug, Default)]
pub struct RemoteProbe {
    /// 远端文件大小，无法确认时为 `None`
    pub length: Option<u64>,
    /// 是否支持 Range 分段请求
    pub ranged: bool,
    /// `etag` 响应头，没有时为空串
    pub etag: String,
}

impl RemoteProbe {
    /// 是否可以按分片下载。
    pub fn chunkable(&self, threads: usize) -> bool {
        matches!(self.length, Some(length) if self.ranged && threads > 1 && length >= MIN_CHUNK)
    }
}

/// 下载结果。
#[derive(Clone, Debug)]
pub struct DownloadOutcome {
    /// 写入的字节数
    pub written: u64,
    /// 是否走了分片下载
    pub chunked: bool,
}

impl FetchClient {
    /// 探测远端文件大小、是否支持分段，以及用于判断是否变更的 etag。
    ///
    /// HEAD 不可用时退回一次 `Range: bytes=0-0` 的 GET，从响应头读同样的信息。
    pub async fn probe(&self, url: &str) -> RemoteProbe {
        let request = with_ua(FetchRequest::new(url));
        if let Some(probe) = probe_head(&request).await {
            return probe;
        }

        probe_range(&request).await.unwrap_or_default()
    }

    /// 下载 `url` 到 `path`，必要时按 `threads` 分片并行。
    pub async fn download(
        &self,
        url: &str,
        path: impl AsRef<Path>,
        threads: usize,
    ) -> Result<DownloadOutcome> {
        let probe = self.probe(url).await;
        self.download_probed(url, path, threads, probe).await
    }

    /// 用已知的探测结果下载，避免同一个地址被探测两次。
    pub async fn download_probed(
        &self,
        url: &str,
        path: impl AsRef<Path>,
        threads: usize,
        probe: RemoteProbe,
    ) -> Result<DownloadOutcome> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .with_context(|| format!("创建下载目录失败: {}", parent.display()))?;
        }

        let request = with_ua(FetchRequest::new(url));
        let threads = threads.max(1);
        if !probe.chunkable(threads) {
            return download_single(request, path).await;
        }

        let total = probe.length.expect("分片下载要求已知文件大小");
        download_chunks(request, path, threads, total).await
    }

    /// 用一次 GET 顺序下载，已知 etag 时一并返回。
    pub async fn download_single_etag(
        &self,
        url: &str,
        path: impl AsRef<Path>,
    ) -> Result<(DownloadOutcome, RemoteProbe)> {
        let probe = self.probe(url).await;
        let outcome = self.download_probed(url, path, 1, probe.clone()).await?;
        Ok((outcome, probe))
    }
}

/// 单线程顺序下载。
async fn download_single(request: FetchRequest, path: &Path) -> Result<DownloadOutcome> {
    let mut file = File::create(path)
        .await
        .with_context(|| format!("创建文件失败: {}", path.display()))?;
    let mut stream = backend::get_stream(request).await?;
    if !stream.is_success() {
        bail!("下载失败，状态码 {}", stream.status());
    }

    let written = write_stream(&mut file, &mut stream).await?;
    file.flush().await.context("刷新文件失败")?;

    Ok(DownloadOutcome {
        written,
        chunked: false,
    })
}

/// 按分片并行下载，每块写入文件的对应偏移。
async fn download_chunks(
    request: FetchRequest,
    path: &Path,
    threads: usize,
    total: u64,
) -> Result<DownloadOutcome> {
    let chunk = total.div_ceil(threads as u64);
    // 先按最终大小建好文件, 各分片才能直接写自己的偏移
    File::create(path)
        .await
        .with_context(|| format!("创建文件失败: {}", path.display()))?
        .set_len(total)
        .await
        .context("预分配文件大小失败")?;

    let mut handles = Vec::with_capacity(threads);
    for index in 0..threads {
        let start = index as u64 * chunk;
        if start >= total {
            break;
        }
        let end = (start + chunk).min(total) - 1;

        let request = request
            .clone()
            .with("range", format!("bytes={start}-{end}"));
        let path = path.to_path_buf();
        handles.push(tokio::spawn(async move {
            download_chunk(request, &path, start, end).await
        }));
    }

    let mut written = 0;
    for handle in handles {
        written += handle.await.context("分片下载任务异常")??;
    }

    Ok(DownloadOutcome {
        written,
        chunked: true,
    })
}

/// 下载一个分片并写入文件的对应偏移。
async fn download_chunk(request: FetchRequest, path: &Path, start: u64, end: u64) -> Result<u64> {
    let mut stream = backend::get_stream(request).await?;
    if !stream.is_success() {
        bail!("分片下载失败，状态码 {}", stream.status());
    }

    let mut file = OpenOptions::new()
        .write(true)
        .open(path)
        .await
        .with_context(|| format!("打开文件失败: {}", path.display()))?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .context("定位写入位置失败")?;

    let written = write_stream(&mut file, &mut stream).await?;
    file.flush().await.context("刷新文件失败")?;

    if written != end - start + 1 {
        bail!(
            "分片长度不符，期望 {} 字节，实际 {written} 字节",
            end - start + 1
        );
    }

    Ok(written)
}

/// 把响应体逐块写入文件，返回写入的字节数。
async fn write_stream(file: &mut File, stream: &mut backend::FetchStream) -> Result<u64> {
    let mut written = 0;
    while let Some(chunk) = stream.next().await {
        let chunk: Bytes = chunk.context("读取响应内容失败")?;
        file.write_all(&chunk).await.context("写入文件失败")?;
        written += chunk.len() as u64;
    }
    Ok(written)
}

/// 用 HEAD 探测，HEAD 不可用时返回 `None` 交给 Range 探测。
async fn probe_head(request: &FetchRequest) -> Option<RemoteProbe> {
    let response = backend::head(request.clone()).await.ok()?;
    if !response.is_success() {
        return None;
    }

    Some(RemoteProbe {
        length: response
            .header("content-length")
            .and_then(|value| value.trim().parse().ok()),
        ranged: accepts_ranges(response.header("accept-ranges").map(String::as_str)),
        etag: response.header("etag").cloned().unwrap_or_default(),
    })
}

/// 用一次 `Range: bytes=0-0` 的 GET 探测，从 `Content-Range` 读总大小。
///
/// 服务器忽略 Range 时会返回整个文件，此时仍能按响应头判断大小与是否支持分段。
async fn probe_range(request: &FetchRequest) -> Option<RemoteProbe> {
    let request = request.clone().with("range", "bytes=0-0");
    let stream = backend::get_stream(request).await.ok()?;
    if !stream.is_success() {
        return None;
    }

    let ranged = accepts_ranges(stream.header("accept-ranges").map(String::as_str));
    let length = stream
        .header("content-range")
        .and_then(|value| total_of_content_range(value))
        .or_else(|| {
            stream
                .header("content-length")
                .and_then(|value| value.trim().parse().ok())
        });
    let etag = stream.header("etag").cloned().unwrap_or_default();

    Some(RemoteProbe {
        length,
        ranged,
        etag,
    })
}

/// 响应头是否声明支持分段。
fn accepts_ranges(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.to_ascii_lowercase().contains("bytes"))
}

/// 从 `Content-Range: bytes 0-0/12345` 里取总大小。
fn total_of_content_range(value: &str) -> Option<u64> {
    value.rsplit('/').next()?.trim().parse().ok()
}
