use std::collections::VecDeque;
use std::time::Instant;

/// 窗口内保留的采样点数量，按一秒一个点约为一分钟。
const CAPACITY: usize = 60;

/// 图表使用的一个采样点。
#[derive(Clone)]
pub struct Point {
    /// 距离现在多少秒，最新点为 0。
    pub ago: u64,
    /// 上传速率，单位 Bytes/s。
    pub upload: f64,
    /// 下载速率，单位 Bytes/s。
    pub download: f64,
    /// 内存占用，单位 Bytes。
    pub memory: f64,
}

/// 内部保存的采样点，`ago` 在读取时按位置推算，不落库。
struct Sample {
    upload: f64,
    download: f64,
    memory: f64,
}

/// 实时监控数据。
///
/// 数据来自内核 gRPC API 推送的运行状态：内存占用直接取内核进程占用，
/// 上下行速率由累计流量的两次采样差除以实际间隔得到，累计流量本身取内核的累计值。
pub struct Monitor {
    samples: VecDeque<Sample>,
    total_upload: f64,
    total_download: f64,
    /// 上一次采样时的累计流量与时刻，用于换算速率。
    last: Option<(u64, u64, Instant)>,
}

impl Monitor {
    /// 创建空的监控数据，内核启动后由采样逐步填充。
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(CAPACITY),
            total_upload: 0.,
            total_download: 0.,
            last: None,
        }
    }

    /// 清空监控数据，内核停止后调用，避免下次启动残留上一轮曲线。
    pub fn reset(&mut self) {
        self.samples.clear();
        self.total_upload = 0.;
        self.total_download = 0.;
        self.last = None;
    }

    /// 用一个内核状态快照推进一次采样。
    ///
    /// `upload_total`、`download_total` 是内核启动以来的累计流量，`memory` 是内核进程占用。
    pub fn push(&mut self, upload_total: u64, download_total: u64, memory: u64) {
        let now = Instant::now();
        // 首个采样没有参照，速率按 0 处理；之后按实际间隔换算。
        let (upload, download) = match self.last {
            Some((last_upload, last_download, last_at)) => {
                let seconds = now.duration_since(last_at).as_secs_f64().max(0.001);
                (
                    upload_total.saturating_sub(last_upload) as f64 / seconds,
                    download_total.saturating_sub(last_download) as f64 / seconds,
                )
            }
            None => (0., 0.),
        };

        self.last = Some((upload_total, download_total, now));
        self.total_upload = upload_total as f64;
        self.total_download = download_total as f64;
        self.samples.push_back(Sample {
            upload,
            download,
            memory: memory as f64,
        });
        while self.samples.len() > CAPACITY {
            self.samples.pop_front();
        }
    }

    /// 当前窗口的全部采样点，从最早到最新。
    pub fn points(&self) -> Vec<Point> {
        let count = self.samples.len();
        self.samples
            .iter()
            .enumerate()
            .map(|(index, sample)| Point {
                ago: (count - 1 - index) as u64,
                upload: sample.upload,
                download: sample.download,
                memory: sample.memory,
            })
            .collect()
    }

    /// 最新的上传速率，单位 Bytes/s。
    pub fn upload(&self) -> f64 {
        self.samples
            .back()
            .map(|sample| sample.upload)
            .unwrap_or(0.)
    }

    /// 最新的下载速率，单位 Bytes/s。
    pub fn download(&self) -> f64 {
        self.samples
            .back()
            .map(|sample| sample.download)
            .unwrap_or(0.)
    }

    /// 累计上传量，单位 Bytes。
    pub fn total_upload(&self) -> f64 {
        self.total_upload
    }

    /// 累计下载量，单位 Bytes。
    pub fn total_download(&self) -> f64 {
        self.total_download
    }
}
