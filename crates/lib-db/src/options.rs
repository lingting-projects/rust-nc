use std::path::PathBuf;

/// 嵌入式 SQLite 的连接参数。
#[derive(Debug, Clone)]
pub struct DatabaseOptions {
    /// 数据库目录，SQLite 数据库文件落在该目录下。
    pub path: PathBuf,
    /// 连接池上限；SQLite 在 WAL 模式下允许并发读，写入仍然串行。
    pub max_connections: u32,
}

impl DatabaseOptions {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            max_connections: 5,
        }
    }
}
