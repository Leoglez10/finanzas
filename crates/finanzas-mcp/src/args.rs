use chrono::NaiveDate;
use serde_json::Value;
use crate::tools::ToolError;

pub struct Args<'a>(pub &'a Value);
impl Args<'_> {
    fn present(&self, key: &str) -> Option<&Value> { self.0.get(key).filter(|v| !v.is_null()) }
    pub fn optional_string(&self, key: &str) -> Result<Option<&str>, ToolError> {
        self.present(key).map(|v| v.as_str().ok_or_else(|| ToolError(format!("{key}: expected a string")))).transpose()
    }
    pub fn required_string(&self, key: &str) -> Result<&str, ToolError> {
        self.optional_string(key)?.filter(|s| !s.is_empty()).ok_or_else(|| ToolError(format!("{key}: required")))
    }
    pub fn bool(&self, key: &str) -> Result<Option<bool>, ToolError> {
        self.present(key).map(|v| v.as_bool().ok_or_else(|| ToolError(format!("{key}: expected true or false")))).transpose()
    }
    pub fn int(&self, key: &str, default: i64, min: i64, max: i64) -> Result<i64, ToolError> {
        if self.present(key).is_none() { return Ok(default); }
        let n = self.cents(key).ok().filter(|n| n % 100 == 0).map(|n| n / 100)
            .ok_or_else(|| ToolError(format!("{key}: expected an integer from {min} to {max}")))?;
        if !(min..=max).contains(&n) { return Err(ToolError(format!("{key}: expected an integer from {min} to {max}"))); }
        Ok(n)
    }
    pub fn date(&self, key: &str) -> Result<NaiveDate, ToolError> {
        let v = self.present(key).ok_or_else(|| ToolError(format!("{key}: required")))?;
        let text = v.as_str().unwrap_or("");
        NaiveDate::parse_from_str(text, "%Y-%m-%d").ok().filter(|d| text.len() == 10 && d.format("%Y-%m-%d").to_string() == text)
            .ok_or_else(|| ToolError(format!("{key}: expected YYYY-MM-DD")))
    }
    pub fn range(&self) -> Result<(NaiveDate, NaiveDate), ToolError> {
        let (start, end) = (self.date("startDate")?, self.date("endDate")?);
        if start > end { return Err(ToolError("startDate must be on or before endDate".into())); }
        Ok((start, end))
    }
    pub fn month(&self, key: &str) -> Result<i32, ToolError> {
        let text = self.present(key).and_then(Value::as_str).unwrap_or("");
        let b = text.as_bytes();
        if b.len() != 7 || b[4] != b'-' || !b.iter().enumerate().all(|(i, c)| i == 4 || c.is_ascii_digit()) {
            return Err(ToolError(format!("{key}: expected YYYY-MM")));
        }
        let y: i32 = text[..4].parse().unwrap();
        let m: i32 = text[5..].parse().unwrap();
        if !(1..=12).contains(&m) { return Err(ToolError(format!("{key}: expected YYYY-MM"))); }
        Ok(y * 100 + m)
    }
    pub fn cents(&self, key: &str) -> Result<i64, ToolError> {
        let v = self.present(key).ok_or_else(|| ToolError(format!("{key}: required")))?;
        let n = v.as_number().ok_or_else(|| ToolError(format!("{key}: expected a number")))?;
        let text = n.to_string();
        let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text.as_str(), "0"));
        let exponent = exponent.parse::<i64>().unwrap_or_else(|_| if exponent.starts_with('-') { i64::MIN / 2 } else { i64::MAX / 2 });
        let negative = mantissa.starts_with('-');
        let unsigned = mantissa.trim_start_matches('-');
        let (whole, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        let digits = format!("{whole}{frac}");
        let significant = digits.trim_start_matches('0');
        if significant.is_empty() { return Ok(0); }
        let magnitude = (significant.len() as i64).saturating_add(exponent).saturating_sub(frac.len() as i64);
        if magnitude > 13 || (magnitude == 13 && (significant.as_bytes()[0] > b'1' || significant.as_bytes()[1..].iter().any(|c| *c != b'0'))) {
            return Err(ToolError(format!("{key}: must be between -1000000000000 and 1000000000000")));
        }
        let scale = exponent.saturating_sub(frac.len() as i64).saturating_add(2);
        let cents = if scale >= 0 {
            significant.parse::<i64>().ok().and_then(|n| u32::try_from(scale).ok().and_then(|s| 10_i64.checked_pow(s).and_then(|m| n.checked_mul(m))))
        } else {
            let places = scale.unsigned_abs();
            if places > significant.len() as u64 { None } else {
                let cut = significant.len() - places as usize;
                if significant[cut..].bytes().any(|c| c != b'0') { None } else { Some(if cut == 0 { 0 } else { significant[..cut].parse::<i64>().unwrap() }) }
            }
        }.ok_or_else(|| ToolError(format!("{key}: at most 2 decimals")))?;
        Ok(if negative { -cents } else { cents })
    }
    pub fn id(&self, key: &str, entity: &str) -> Result<Option<i64>, ToolError> {
        let Some(s) = self.optional_string(key)? else { return Ok(None) };
        let n = s.strip_prefix(entity).and_then(|s| s.strip_prefix("/p")).filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())).and_then(|s| s.parse::<i64>().ok()).filter(|n| *n > 0);
        n.map(Some).ok_or_else(|| ToolError(format!("{key}: not found")))
    }
}
