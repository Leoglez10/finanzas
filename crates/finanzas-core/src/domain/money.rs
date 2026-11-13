//! Money input and formatting.

use crate::models::Cents;

pub fn cents_from(input: &str) -> Option<Cents> {
    let cleaned: String = input.chars().filter(|c| !matches!(c, ' ' | '$' | ',' | '-')).collect();
    let mut parts = cleaned.split('.');
    let whole = parts.next()?;
    let fraction = parts.next();
    if parts.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) { return None; }
    let mut value = whole.parse::<Cents>().ok()?.checked_mul(100)?;
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) { return None; }
        let digits = fraction.as_bytes();
        value = value.checked_add(i64::from(digits[0] - b'0') * 10)?;
        if digits.len() > 1 { value = value.checked_add(i64::from(digits[1] - b'0'))?; }
        if digits.len() > 2 && digits[2] >= b'5' { value = value.checked_add(1)?; }
    }
    Some(value)
}

/// A leading `+`/`-` adjusts `current`; anything else sets it. The amount may be arithmetic:
/// `550*2`, `100/4`, `(200+50)*3`.
pub fn budgeted_from(input: &str, current: Cents) -> Option<Cents> {
    let text = input.trim();
    if text.is_empty() { return Some(0); }
    let (sign, rest) = match text.chars().next() { Some(c @ ('+' | '-')) => (Some(c), &text[1..]), _ => (None, text) };
    let value = eval_cents(rest).filter(|v| *v >= 0)?;
    match sign {
        Some('+') => current.checked_add(value),
        Some('-') => Some(current.saturating_sub(value).max(0)),
        _ => Some(value),
    }
}

/// A typed amount: blank is 0, a leading sign is ignored (the form picks expense or income),
/// and arithmetic works (`550*2`). Never negative.
pub fn amount_from(input: &str) -> Option<Cents> {
    let text = input.trim();
    if text.is_empty() { return Some(0); }
    eval_cents(text.strip_prefix(['-', '+']).unwrap_or(text)).filter(|v| *v >= 0)
}

/// Evaluates `+ - * x /` and parentheses over amounts, in cents, rounding half up at each
/// division or product. `$`, `,` and spaces are ignored.
pub fn eval_cents(input: &str) -> Option<Cents> {
    let chars: Vec<char> = input.chars().filter(|c| !matches!(c, ' ' | '$' | ',')).collect();
    let mut p = Parser { chars, at: 0 };
    let value = p.expr()?;
    (p.at == p.chars.len()).then_some(value).and_then(|v| Cents::try_from(v).ok())
}

struct Parser { chars: Vec<char>, at: usize }
impl Parser {
    fn peek(&self) -> Option<char> { self.chars.get(self.at).copied() }
    fn expr(&mut self) -> Option<i128> {
        let mut value = self.term()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.at += 1;
            let rhs = self.term()?;
            value = if op == '+' { value.checked_add(rhs)? } else { value.checked_sub(rhs)? };
        }
        Some(value)
    }
    fn term(&mut self) -> Option<i128> {
        let mut value = self.factor()?;
        while let Some(op @ ('*' | 'x' | 'X' | '/')) = self.peek() {
            self.at += 1;
            let rhs = self.factor()?;
            value = if op == '/' {
                if rhs == 0 { return None; }
                div_round(value.checked_mul(100)?, rhs)
            } else { div_round(value.checked_mul(rhs)?, 100) };
        }
        Some(value)
    }
    fn factor(&mut self) -> Option<i128> {
        match self.peek()? {
            '(' => { self.at += 1; let v = self.expr()?; (self.peek() == Some(')')).then(|| { self.at += 1; v }) }
            '-' => { self.at += 1; self.factor()?.checked_neg() }
            _ => {
                let start = self.at;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') { self.at += 1; }
                let number: String = self.chars[start..self.at].iter().collect();
                cents_from(&number).map(i128::from)
            }
        }
    }
}
fn div_round(n: i128, d: i128) -> i128 { let (q, r) = (n / d, n % d); if 2 * r.abs() >= d.abs() { q + n.signum() * d.signum() } else { q } }

pub fn format(cents: Cents) -> String {
    let magnitude = cents.unsigned_abs();
    let whole = (magnitude / 100).to_string();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) { grouped.push(','); }
        grouped.push(ch);
    }
    format!("{}${}.{:02}", if cents < 0 { "-" } else { "" }, grouped, magnitude % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_amounts_to_positive_cents() {
        for (text, expected) in [("125.50", 12_550), ("$1,250.5", 125_050), (" 80 ", 8_000), ("0.07", 7), ("-40", 4_000), ("12.345", 1_235)] {
            assert_eq!(cents_from(text), Some(expected));
        }
    }

    #[test]
    fn rejects_invalid_amounts() {
        for text in ["", "abc", "1.2.3"] { assert_eq!(cents_from(text), None); }
    }

    #[test]
    fn budget_input_sets_or_adjusts_the_assigned_amount() {
        for (text, current, expected) in [("500", 20_000, 50_000), ("+100", 20_000, 30_000), ("-50", 20_000, 15_000), ("- 300", 20_000, 0), ("+$1,000.50", 0, 100_050)] {
            assert_eq!(budgeted_from(text, current), Some(expected));
        }
    }

    #[test]
    fn budget_input_rejects_invalid_text() { assert_eq!(budgeted_from("+abc", 20_000), None); }

    #[test]
    fn typed_amounts_do_math_and_blank_is_zero() {
        for (text, expected) in [("", 0), ("  ", 0), ("550*2", 110_000), ("-40", 4_000), ("+12.5", 1_250), ("$1,000/4", 25_000), ("(10+5)*2", 3_000)] {
            assert_eq!(amount_from(text), Some(expected), "{text}");
        }
        for text in ["abc", "5/0", "10-20", "2*"] { assert_eq!(amount_from(text), None, "{text}"); }
        assert_eq!(budgeted_from("", 20_000), Some(0), "a blank budget is 0");
    }

    #[test]
    fn budget_input_does_math() {
        for (text, current, expected) in [("550*2", 0, 110_000), ("550 x 2", 0, 110_000), ("100/4", 0, 2_500), ("(200+50)*3", 0, 75_000), ("100-20", 50_000, 8_000),
            ("10/3", 0, 333), ("2*1.5", 0, 300), ("$1,000*2", 0, 200_000), ("+550*2", 10_000, 120_000), ("-25*2", 20_000, 15_000), ("1+2*3", 0, 700)] {
            assert_eq!(budgeted_from(text, current), Some(expected), "{text}");
        }
        for text in ["5/0", "20-50", "2*", "(1+2", "1+abc", "*3"] { assert_eq!(budgeted_from(text, 0), None, "{text}"); }
    }

    #[test]
    fn formatting_and_rounding_boundaries() {
        assert_eq!(format(123456), "$1,234.56");
        assert_eq!(format(-123456), "-$1,234.56");
        assert_eq!(cents_from("1.0049"), Some(100));
        assert_eq!(cents_from("1.999"), Some(200));
        assert_eq!(cents_from("1."), None);
        assert_eq!(cents_from("999999999999999999999999"), None);
    }
}
