use crate::core::{AnyResult, BizError};
use encoding::{DecoderTrap, Encoding};

#[cfg(target_os = "windows")]
pub fn get_system() -> AnyResult<String> {
    use winapi::um::winnls::GetACP;

    let str = unsafe {
        let code_page = GetACP();

        match code_page {
            936 => "gbk",
            65001 => "utf-8",
            1252 => "windows-1252",
            950 => "big5",
            1251 => "windows-1251",
            _ => return Err(Box::new(BizError::CharsetReadErr)),
        }
    };

    Ok(str.to_string())
}

pub fn convert(bytes: Vec<u8>, charset: &str) -> AnyResult<String> {
    let encoding: &dyn Encoding = match charset {
        "utf-8" => encoding::all::UTF_8,
        "gbk" => encoding::all::GBK,
        "big5-2003" => encoding::all::BIG5_2003,
        "windows-1252" => encoding::all::WINDOWS_1252,
        "iso-8859-1" => encoding::all::ISO_8859_1,
        _ => {
            log::warn!("未知字符集[{}], 尝试使用UTF-8解码", charset);
            encoding::all::UTF_8
        }
    };

    let string = encoding.decode(&bytes, DecoderTrap::Replace)?;
    Ok(string)
}
