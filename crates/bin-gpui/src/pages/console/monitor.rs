use std::collections::VecDeque;

/// 窗口内保留的采样点数量，按一秒一个点约为一分钟。
const CAPACITY: usize = 60;

/// 上传速率的基准值，单位 Bytes/s。
const UPLOAD_BASE: f64 = 820. * 1024.;
/// 下载速率的基准值，单位 Bytes/s。
const DOWNLOAD_BASE: f64 = 1_640. * 1024.;
/// 内存占用的基准值，单位 Bytes。
const MEMORY_BASE: f64 = 286. * 1024. * 1024.;

/// 启动前的累计流量基数，让总量一上来就有意义。
const TOTAL_UPLOAD_BASE: f64 = 2.1 * 1024. * 1024. * 1024.;
const TOTAL_DOWNLOAD_BASE: f64 = 7.8 * 1024. * 1024. * 1024.;

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
/// 内核监控的接入方式尚未确定，这里先用随机游走生成上传、下载与内存三条曲线，
/// 让界面与交互先成型；接入真实数据时替换本模块即可，页面不需要改动。
pub struct Monitor {
    samples: VecDeque<Sample>,
    total_upload: f64,
    total_download: f64,
    jitter: Jitter,
}

impl Monitor {
    /// 创建监控数据，并预填满整个窗口，图表一开始就是完整的。
    pub fn new() -> Self {
        let mut monitor = Self {
            samples: VecDeque::with_capacity(CAPACITY),
            total_upload: TOTAL_UPLOAD_BASE,
            total_download: TOTAL_DOWNLOAD_BASE,
            jitter: Jitter::new(0x9E37_79B9_7F4A_7C15),
        };

        for _ in 0..CAPACITY {
            monitor.tick();
        }

        monitor
    }

    /// 前进一秒，产生一个新采样点。
    pub fn tick(&mut self) {
        let last = self.samples.back();
        let (upload, download, memory) = match last {
            Some(last) => (
                walk(last.upload, UPLOAD_BASE, 0.25, &mut self.jitter),
                walk(last.download, DOWNLOAD_BASE, 0.25, &mut self.jitter),
                walk(last.memory, MEMORY_BASE, 0.03, &mut self.jitter),
            ),
            None => (UPLOAD_BASE, DOWNLOAD_BASE, MEMORY_BASE),
        };

        self.total_upload += upload;
        self.total_download += download;
        self.samples.push_back(Sample {
            upload,
            download,
            memory,
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

/// 以 `base` 为中心做一次小幅游走，结果不会低于 0。
fn walk(current: f64, base: f64, amplitude: f64, jitter: &mut Jitter) -> f64 {
    // 向基准值回拉，避免长时间游走偏出合理区间。
    let pull = (base - current) * 0.1;
    (current + pull + base * amplitude * jitter.signed()).max(0.)
}

/// 模拟数据的随机源，用 xorshift 避免为 mock 引入额外依赖。
struct Jitter(u64);

impl Jitter {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// 返回 `[0, 1)` 内的随机数。
    fn next(&mut self) -> f64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value >> 11) as f64 / (1u64 << 53) as f64
    }

    /// 返回 `[-1, 1)` 内的随机数。
    fn signed(&mut self) -> f64 {
        self.next() * 2. - 1.
    }
}
