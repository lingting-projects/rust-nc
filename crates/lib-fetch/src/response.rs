use bytes::Bytes;
use lib_core::MultiStringValue;

/// 响应信息。
#[derive(Debug, Clone, Default)]
pub struct FetchResponse {
    /// 响应体
    pub body: Bytes,
    /// 响应头
    pub headers: MultiStringValue,
    /// 状态码
    pub status: u16,
}

impl FetchResponse {
    /// 由响应体、响应头与状态码构建。
    pub fn new(body: Bytes, headers: MultiStringValue, status: u16) -> Self {
        Self {
            body,
            headers,
            status,
        }
    }

    /// 是否请求成功。
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 响应体转字符串, 非法 utf-8 时使用替换字符。
    pub fn string(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// 读取响应头, 返回第一个值。
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get_first(name)
    }

    /// `content-type` 响应头。
    pub fn content_type(&self) -> Option<&String> {
        self.headers.content_type()
    }

    /// 不含参数的 `content-type`。
    pub fn mime_type(&self) -> Option<String> {
        self.headers.mime_type()
    }

    /// `content-length` 响应头。
    pub fn content_length(&self) -> Option<u64> {
        self.header("content-length")?.parse().ok()
    }

    /// `subscription-userinfo` 响应头。
    pub fn subscription_userinfo(&self) -> Option<&String> {
        self.header("subscription-userinfo")
    }

    /// `content-disposition` 响应头。
    pub fn content_disposition(&self) -> Option<&String> {
        self.header("content-disposition")
    }

    /// 响应体是否为空。
    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }

    /// 响应体长度。
    pub fn len(&self) -> usize {
        self.body.len()
    }
}
