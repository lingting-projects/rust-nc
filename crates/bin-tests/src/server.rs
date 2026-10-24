//! 假订阅服务。
//!
//! 用本地 http 服务替代真实订阅源: 每个订阅源在 `assets/source` 下是一个文件,
//! 服务按路径返回对应文件的内容, 并带上 `Subscription-Userinfo` 响应头。
//! 这样订阅转换测试不依赖外网, 也不受源失效影响。
//!
//! 路径约定:
//!
//! - `GET /{name}`: 返回 `assets/source/{name}` 的内容;
//! - `GET /`: 返回全部源的文件名, 便于排查;
//! - 其他路径返回 404。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::thread;

use anyhow::{Context, Result, bail};

use crate::source::{self, Source};

/// 订阅流量与到期信息, 由假服务通过响应头返回。
///
/// 数值固定, 便于校验转换结果的响应头是否被正确转发。
pub const USER_INFO: &str =
    "download=1073741824; upload=536870912; total=107374182400; expire=1893456000";

/// 本地假订阅服务。
pub struct Server {
    /// 访问地址, 不含末尾斜杠
    base: String,
}

impl Server {
    /// 启动服务并返回访问地址。
    ///
    /// 服务在后台线程中监听随机端口, 随进程退出而结束; 不单独提供停止方法,
    /// 因为它不持有需要回收的资源, 端口在进程结束时由系统释放。
    pub fn start() -> Result<Self> {
        let sources = source::load_all()?;
        if sources.is_empty() {
            bail!("未找到订阅源, 请把源文件放入 {}", source::SOURCE_DIR);
        }

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("假订阅服务监听失败")?;
        let port = listener
            .local_addr()
            .context("读取假订阅服务监听地址失败")?
            .port();

        thread::Builder::new()
            .name("fake-subscribe".to_string())
            .spawn(move || serve(listener, sources))
            .context("启动假订阅服务线程失败")?;

        Ok(Self {
            base: format!("http://{}:{port}", Ipv4Addr::LOCALHOST),
        })
    }

    /// 服务地址, 不含末尾斜杠。
    pub fn base(&self) -> &str {
        &self.base
    }

    /// 指定源的订阅地址。
    pub fn url(&self, name: &str) -> String {
        format!("{}/{}", self.base, name)
    }
}

/// 循环接收连接, 每个连接独立处理。
fn serve(listener: TcpListener, sources: Vec<Source>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // 单个连接异常不影响服务继续接收
                if let Err(error) = handle(stream, &sources) {
                    log::warn!("[server] 处理请求失败: {error}");
                }
            }
            Err(error) => log::warn!("[server] 接收连接失败: {error}"),
        }
    }
}

/// 处理单个连接: 读取请求行与请求头, 按路径返回内容。
fn handle(mut stream: TcpStream, sources: &[Source]) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let path = request(&mut reader)?;

    match find(sources, &path) {
        Some(source) => write_ok(&mut stream, &source.content)?,
        None => write_not_found(&mut stream)?,
    }

    Ok(())
}

/// 读取请求行并消费请求头, 返回请求路径。
///
/// 请求头必须按空行读完: 只读请求行会让客户端仍在发送头部时就去写响应,
/// 而且连接上残留的数据会影响后续读取。
fn request(reader: &mut BufReader<TcpStream>) -> Result<String> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let path = line
        .split_whitespace()
        .nth(1)
        .context("请求行格式异常")?
        .to_string();

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line.trim().is_empty() {
            break;
        }
    }

    Ok(path)
}

/// 按请求路径查找源, 忽略查询参数与前导斜杠。
fn find<'a>(sources: &'a [Source], path: &str) -> Option<&'a Source> {
    let name = path.split('?').next()?.trim_start_matches('/');
    sources.iter().find(|source| source.name == name)
}

/// 返回订阅内容。
fn write_ok(stream: &mut TcpStream, body: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         content-type: text/plain; charset=utf-8\r\n\
         subscription-userinfo: {USER_INFO}\r\n\
         content-length: {}\r\n\
         connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

/// 返回 404。
fn write_not_found(stream: &mut TcpStream) -> Result<()> {
    let body = "not found";
    let response = format!(
        "HTTP/1.1 404 Not Found\r\n\
         content-type: text/plain; charset=utf-8\r\n\
         content-length: {}\r\n\
         connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}
