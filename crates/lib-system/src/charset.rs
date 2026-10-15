use anyhow::{Result, bail};
use encoding::{DecoderTrap, Encoding};

/// 获取当前系统的活动代码页名称。
#[cfg(target_os = "windows")]
pub fn system() -> Result<String> {
    use winapi::um::winnls::GetACP;

    let name = unsafe {
        match GetACP() {
            936 => "gbk",
            65001 => "utf-8",
            1252 => "windows-1252",
            950 => "big5",
            1251 => "windows-1251",
            _ => bail!("无法识别的系统代码页"),
        }
    };

    Ok(name.to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn system() -> Result<String> {
    Ok("utf-8".to_string())
}

/// 按指定字符集解码字节内容，未知字符集时回退到 UTF-8。
pub fn convert(bytes: Vec<u8>, charset: &str) -> Result<String> {
    let encoding: &dyn Encoding = match charset {
        "utf-8" => encoding::all::UTF_8,
        "gbk" => encoding::all::GBK,
        "big5-2003" => encoding::all::BIG5_2003,
        "windows-1252" => encoding::all::WINDOWS_1252,
        "iso-8859-1" => encoding::all::ISO_8859_1,
        _ => {
            log::warn!("未知字符集[{charset}], 尝试使用UTF-8解码");
            encoding::all::UTF_8
        }
    };

    let string = encoding
        .decode(&bytes, DecoderTrap::Replace)
        .map_err(|error| anyhow::anyhow!("按字符集 {charset} 解码失败: {error}"))?;
    Ok(string)
}
