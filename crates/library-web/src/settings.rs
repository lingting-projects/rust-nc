use crate::tbl_setting::TblSettingSoftware;
use std::sync::LazyLock;

pub static is_minimize: LazyLock<bool> = LazyLock::new(|| TblSettingSoftware::is_minimize());
