// params.rs — 表单参数宽松解析
// 经验：空字符串不应导致反序列化失败（见会话日志 2026-08-09-2326），
// 所有数值字段统一走这里：空字符串 = 不修改（None）

use crate::error::ApiError;

/// 可选整数：空字符串视为未设置
pub fn parse_optional_i64(
    input: Option<String>,
    field: &str,
) -> Result<Option<i64>, ApiError> {
    let Some(value) = input else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    value
        .trim()
        .parse()
        .map(Some)
        .map_err(|_| ApiError::bad_request(format!("{field} 必须是整数")))
}

/// 可选整数（含清空语义）：传空字符串返回 Clear 表示要把字段置空
#[derive(Debug, Clone, PartialEq)]
pub enum Clearable<T> {
    Keep,
    Clear,
    Set(T),
}

pub fn parse_clearable_i64(
    input: Option<String>,
    field: &str,
) -> Result<Clearable<i64>, ApiError> {
    match input {
        None => Ok(Clearable::Keep),
        Some(value) if value.trim().is_empty() => Ok(Clearable::Clear),
        Some(value) => value
            .trim()
            .parse()
            .map(Clearable::Set)
            .map_err(|_| ApiError::bad_request(format!("{field} 必须是整数"))),
    }
}

/// 可选字符串（含清空语义）：None=未提供（不修改）；
/// Some(空串)=清空字段为 None；Some(非空)=设置新值
pub fn parse_optional_string(input: Option<String>) -> Option<Option<String>> {
    input.map(|s| {
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    })
}
