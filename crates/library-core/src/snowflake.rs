use crate::core::BizError;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

/// 雪花算法参数配置
#[derive(Debug, Clone, Copy)]
pub struct SnowflakeParams {
    /// 雪花算法的开始时间戳（自定义）
    pub start_timestamp: u64,
    /// 机器ID所占位数
    pub worker_id_bits: u8,
    /// 数据中心ID所占位数
    pub datacenter_id_bits: u8,
    /// 支持的最大机器ID数量
    pub max_worker_id: u64,
    /// 支持的最大数据中心ID数量
    pub max_datacenter_id: u64,
    /// 序列号所占位数
    pub sequence_bits: u8,
    /// 机器ID左移位数
    pub worker_id_shift: u8,
    /// 数据中心ID左移位数
    pub datacenter_id_shift: u8,
    /// 时间戳左移位数
    pub timestamp_left_shift: u8,
    /// 生成序列号的掩码
    pub sequence_mask: u64,
}

impl SnowflakeParams {
    /// 默认参数配置
    pub const DEFAULT: SnowflakeParams = SnowflakeParams {
        start_timestamp: 1288834974657,
        worker_id_bits: 5,
        datacenter_id_bits: 5,
        max_worker_id: 31,
        max_datacenter_id: 31,
        sequence_bits: 12,
        worker_id_shift: 12,
        datacenter_id_shift: 17,
        timestamp_left_shift: 22,
        sequence_mask: 4095,
    };

    /// 创建新的参数配置
    pub fn new(
        start_timestamp: u64,
        worker_id_bits: u8,
        datacenter_id_bits: u8,
        sequence_bits: u8,
    ) -> Self {
        let max_worker_id = (-1i64 as u64) ^ ((-1i64 as u64) << worker_id_bits);
        let max_datacenter_id = (-1i64 as u64) ^ ((-1i64 as u64) << datacenter_id_bits);
        let worker_id_shift = sequence_bits;
        let datacenter_id_shift = sequence_bits + worker_id_bits;
        let timestamp_left_shift = sequence_bits + worker_id_bits + datacenter_id_bits;
        let sequence_mask = (-1i64 as u64) ^ ((-1i64 as u64) << sequence_bits);

        SnowflakeParams {
            start_timestamp,
            worker_id_bits,
            datacenter_id_bits,
            max_worker_id,
            max_datacenter_id,
            sequence_bits,
            worker_id_shift,
            datacenter_id_shift,
            timestamp_left_shift,
            sequence_mask,
        }
    }
}

/// 雪花算法ID生成器
pub struct Snowflake {
    params: SnowflakeParams,
    worker_id: u64,
    datacenter_id: u64,
    inner: Mutex<SnowflakeInner>,
}

struct SnowflakeInner {
    sequence: u64,
    last_timestamp: u64,
}

impl Snowflake {
    /// 创建新的ID生成器
    pub fn new(
        params: SnowflakeParams,
        worker_id: u64,
        datacenter_id: u64,
    ) -> Result<Self, BizError> {
        if worker_id > params.max_worker_id {
            return Err(BizError::SnowflakeInit(format!(
                "Worker ID cannot be greater than {} or less than 0",
                params.max_worker_id
            )));
        }

        if datacenter_id > params.max_datacenter_id {
            return Err(BizError::SnowflakeInit(format!(
                "Datacenter ID cannot be greater than {} or less than 0",
                params.max_datacenter_id
            )));
        }

        Ok(Snowflake {
            params,
            worker_id,
            datacenter_id,
            inner: Mutex::new(SnowflakeInner {
                sequence: 0,
                last_timestamp: 0,
            }),
        })
    }

    /// 使用默认参数创建ID生成器
    pub fn with_default_params(worker_id: u64, datacenter_id: u64) -> Result<Self, BizError> {
        Self::new(SnowflakeParams::DEFAULT, worker_id, datacenter_id)
    }

    /// 生成下一个ID
    pub fn next_id(&self) -> u64 {
        let mut inner = self.inner.lock().unwrap();
        let mut timestamp = self.current_timestamp();

        // 处理时钟回拨
        if timestamp < inner.last_timestamp {
            if !self.allow_clock_backwards(timestamp) {
                panic!(
                    "Clock moved backwards! current: {}, last: {}",
                    timestamp, inner.last_timestamp
                );
            }
            // 允许回拨，使用上次时间
            timestamp = inner.last_timestamp;
        }

        self.next_id_inner(&mut inner, timestamp)
    }

    /// 生成多个ID
    pub fn next_ids(&self, count: usize) -> Vec<u64> {
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            ids.push(self.next_id());
        }
        ids
    }

    /// 生成下一个ID字符串
    pub fn next_str(&self) -> String {
        self.next_id().to_string()
    }

    /// 生成多个ID字符串
    pub fn next_strs(&self, count: usize) -> Vec<String> {
        self.next_ids(count)
            .into_iter()
            .map(|id| id.to_string())
            .collect()
    }

    /// 依据指定时间戳生成ID
    fn next_id_inner(&self, inner: &mut MutexGuard<SnowflakeInner>, timestamp: u64) -> u64 {
        // 如果是同一时间生成的，则进行毫秒内序列
        if inner.last_timestamp == timestamp {
            inner.sequence = (inner.sequence + 1) & self.params.sequence_mask;
            // 毫秒内序列溢出
            if inner.sequence == 0 {
                // 阻塞到下一个毫秒
                let new_timestamp = self.til_next_millis(inner.last_timestamp);
                inner.last_timestamp = new_timestamp;
                return self.next_id_inner(inner, new_timestamp);
            }
        } else {
            // 时间戳改变，毫秒内序列重置
            inner.sequence = 0;
        }

        // 上次生成ID的时间戳
        inner.last_timestamp = timestamp;

        // 按照规则拼装ID
        ((timestamp - self.params.start_timestamp) << self.params.timestamp_left_shift)
            | (self.datacenter_id << self.params.datacenter_id_shift)
            | (self.worker_id << self.params.worker_id_shift)
            | inner.sequence
    }

    /// 是否允许时钟回拨
    fn allow_clock_backwards(&self, _current_timestamp: u64) -> bool {
        false
    }

    /// 阻塞到下一个毫秒
    fn til_next_millis(&self, last_timestamp: u64) -> u64 {
        let mut timestamp = self.current_timestamp();
        while timestamp <= last_timestamp {
            // 短暂休眠，避免CPU占用过高
            std::thread::sleep(std::time::Duration::from_micros(10));
            timestamp = self.current_timestamp();
        }
        timestamp
    }

    /// 获取当前时间戳(毫秒)
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64
    }
}

static DEFAULT: LazyLock<Mutex<Snowflake>> =
    LazyLock::new(|| Mutex::new(Snowflake::with_default_params(0, 0).unwrap()));

/// 直接设置默认的 Snowflake 实例
pub fn set_default(snowflake: Snowflake) {
    *DEFAULT.lock().unwrap() = snowflake;
}

fn get_default() -> MutexGuard<'static, Snowflake> {
    DEFAULT.lock().unwrap()
}

pub fn next_id() -> u64 {
    get_default().next_id()
}
pub fn next_ids(count: usize) -> Vec<u64> {
    get_default().next_ids(count)
}
pub fn next_str() -> String {
    get_default().next_str()
}
pub fn next_strs(count: usize) -> Vec<String> {
    get_default().next_strs(count)
}
