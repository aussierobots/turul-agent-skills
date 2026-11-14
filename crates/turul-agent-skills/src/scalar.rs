//! Plain-scalar typing: the YAML 1.2 core schema, and the YAML 1.1 types it is
//! compared against. The `typed` flag on [`RawValue::Str`] is derived from both.

use crate::parse::RawValue;

/// A scalar's value under the YAML 1.2 core schema
/// (<https://yaml.org/spec/1.2.2/#1032-tag-resolution>).
///
/// Strings carry no text here: it is the [`RawValue::Str`] `text`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum CoreScalar {
    /// `null`, `Null`, `NULL`, `~` or an empty plain scalar.
    Null,
    /// `true`/`false` in lower, title or upper case.
    Bool(bool),
    /// A decimal, `0o` octal or `0x` hexadecimal integer that fits in `i64`.
    Int(i64),
    /// A core-schema integer outside the `i64` range.
    IntOutOfRange,
    /// A decimal float, `.inf` (signed or not) or `.nan`.
    Float(f64),
    /// Anything else, and every quoted scalar.
    Str,
}

/// A scalar resolved under the YAML 1.2 core schema, with whether a YAML 1.1
/// parser would read the same value.
///
/// # Examples
///
/// ```
/// use turul_agent_skills::{CoreScalar, RawValue};
///
/// let plain = |text: &str| RawValue::Str { text: text.to_string(), typed: true };
///
/// let version = plain("1.0").core_scalar().unwrap();
/// assert_eq!(version.value, CoreScalar::Float(1.0));
/// assert!(version.yaml_1_1_agrees);
///
/// // `yes` is a string in YAML 1.2 but a boolean in YAML 1.1.
/// let legacy = plain("yes").core_scalar().unwrap();
/// assert_eq!(legacy.value, CoreScalar::Str);
/// assert!(!legacy.yaml_1_1_agrees);
///
/// // `0777` is decimal 777 in YAML 1.2 but octal 511 in YAML 1.1.
/// let mode = plain("0777").core_scalar().unwrap();
/// assert_eq!(mode.value, CoreScalar::Int(777));
/// assert!(!mode.yaml_1_1_agrees);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ResolvedScalar {
    /// The YAML 1.2 core-schema value.
    pub value: CoreScalar,
    /// Whether a YAML 1.1 parser reads the same type and value: the YAML 1.1 type
    /// repository (<https://yaml.org/type/>) for null, bool (`y`/`n` included),
    /// int and timestamp, and PyYAML's float form (so `-.5` and `1.2.3` are
    /// strings). `false` for, among others, `yes`/`no`/`on`/`off`/`y`/`n`, a
    /// leading-zero integer such as `0777`, base-60 `1:30`, a timestamp, `1_000`,
    /// `1e3`, and an integer too large for `i128`.
    pub yaml_1_1_agrees: bool,
}

impl RawValue {
    /// This scalar under the YAML 1.2 core schema, and whether YAML 1.1 agrees;
    /// `None` for a sequence or mapping.
    ///
    /// A quoted scalar is always [`CoreScalar::Str`], on which YAML 1.1 agrees.
    pub fn core_scalar(&self) -> Option<ResolvedScalar> {
        match self {
            RawValue::Str { typed: false, .. } => Some(ResolvedScalar {
                value: CoreScalar::Str,
                yaml_1_1_agrees: true,
            }),
            RawValue::Str { text, typed: true } => Some(resolve_plain(text)),
            RawValue::Seq(_) | RawValue::Map(_) => None,
        }
    }
}

/// A plain scalar resolved under both schemas.
fn resolve_plain(text: &str) -> ResolvedScalar {
    let core = core(text);
    let old = yaml_1_1(text);
    let yaml_1_1_agrees = match (core, old) {
        (Core::Null, Old::Null) | (Core::Str, Old::Str) => true,
        (Core::Bool(a), Old::Bool(b)) => a == b,
        (Core::Int(a), Old::Int(Some(b))) => a == Some(b),
        (Core::Float(a), Old::Float(Some(b))) => a == b || (a.is_nan() && b.is_nan()),
        _ => false,
    };
    let value = match core {
        Core::Null => CoreScalar::Null,
        Core::Bool(b) => CoreScalar::Bool(b),
        Core::Int(n) => n
            .and_then(|n| i64::try_from(n).ok())
            .map_or(CoreScalar::IntOutOfRange, CoreScalar::Int),
        Core::Float(f) => CoreScalar::Float(f),
        Core::Str => CoreScalar::Str,
    };
    ResolvedScalar {
        value,
        yaml_1_1_agrees,
    }
}

/// Whether a YAML parser could read the plain scalar as something other than a
/// string: the YAML 1.2 core schema, YAML 1.1, and the looser forms other parsers
/// accept (signed or underscored `0o`/`0x`/`0b`; `_` in any number, with or
/// without an exponent sign; base 60 with a leading zero). Such a scalar is
/// `typed`, and is quoted when it must stay a string.
pub(crate) fn reads_as_non_string(text: &str) -> bool {
    if !matches!(core(text), Core::Str) || !matches!(yaml_1_1(text), Old::Str) {
        return true;
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let prefixed = |prefix: &str, radix: u32| {
        unsigned
            .strip_prefix(prefix)
            .is_some_and(|digits| digits_or_underscores(digits, radix))
    };
    prefixed("0o", 8)
        || prefixed("0x", 16)
        || prefixed("0b", 2)
        || is_underscored_number(unsigned)
        || is_loose_base_60(unsigned)
}

/// Numbers with `_` separators and an optional exponent of either sign:
/// `1_000`, `1_0.5`, `1_0e3`.
fn is_underscored_number(s: &str) -> bool {
    let (mantissa, exponent) = match s.find(['e', 'E']) {
        Some(at) => (&s[..at], Some(&s[at + 1..])),
        None => (s, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    s.contains('_')
        && whole.starts_with(|c: char| c.is_ascii_digit())
        && digits_or_underscores(whole, 10)
        && (fraction.is_empty() || digits_or_underscores(fraction, 10))
        && exponent.is_none_or(|e| is_digits(e.strip_prefix(['-', '+']).unwrap_or(e), 10))
}

/// Base 60 with any leading digit, and an optional fraction: `0:30`, `1:30.5`.
fn is_loose_base_60(s: &str) -> bool {
    let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
    base_60_parts(whole).is_some() && (fraction.is_empty() || digits_or_underscores(fraction, 10))
}

#[derive(Clone, Copy)]
enum Core {
    Null,
    Bool(bool),
    /// `None` when the integer does not fit in `i128`.
    Int(Option<i128>),
    Float(f64),
    Str,
}

/// Core schema tag resolution, <https://yaml.org/spec/1.2.2/#1032-tag-resolution>.
fn core(text: &str) -> Core {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Core::Null,
        "true" | "True" | "TRUE" => return Core::Bool(true),
        "false" | "False" | "FALSE" => return Core::Bool(false),
        ".nan" | ".NaN" | ".NAN" => return Core::Float(f64::NAN),
        _ => {}
    }
    let (negative, unsigned) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return Core::Float(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    if let Some(octal) = text.strip_prefix("0o") {
        if is_digits(octal, 8) {
            return Core::Int(i128::from_str_radix(octal, 8).ok());
        }
        return Core::Str;
    }
    if let Some(hex) = text.strip_prefix("0x") {
        if is_digits(hex, 16) {
            return Core::Int(i128::from_str_radix(hex, 16).ok());
        }
        return Core::Str;
    }
    if is_digits(unsigned, 10) {
        return Core::Int(signed(unsigned.parse::<i128>().ok(), negative));
    }
    if is_decimal_float(unsigned) {
        return Core::Float(text.parse::<f64>().unwrap_or(f64::NAN));
    }
    Core::Str
}

#[derive(Clone, Copy)]
enum Old {
    Null,
    Bool(bool),
    /// `None` when the value does not fit in `i128`.
    Int(Option<i128>),
    /// `None` when the text matches the float form but has no single value.
    Float(Option<f64>),
    Timestamp,
    Str,
}

/// The YAML 1.1 type repository: null, bool, int, float and timestamp.
fn yaml_1_1(text: &str) -> Old {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Old::Null,
        "y" | "Y" | "yes" | "Yes" | "YES" | "true" | "True" | "TRUE" | "on" | "On" | "ON" => {
            return Old::Bool(true);
        }
        "n" | "N" | "no" | "No" | "NO" | "false" | "False" | "FALSE" | "off" | "Off" | "OFF" => {
            return Old::Bool(false);
        }
        ".nan" | ".NaN" | ".NAN" => return Old::Float(Some(f64::NAN)),
        _ => {}
    }
    let (negative, unsigned) = match text.as_bytes()[0] {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return Old::Float(Some(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }));
    }
    if let Some(int) = yaml_1_1_int(unsigned) {
        return Old::Int(signed(int, negative));
    }
    if let Some(float) = yaml_1_1_float(text, unsigned, negative) {
        return Old::Float(float);
    }
    if is_timestamp(text) {
        return Old::Timestamp;
    }
    Old::Str
}

/// `0b[0-1_]+`, `0[0-7_]+`, `0|[1-9][0-9_]*`, `0x[0-9a-fA-F_]+` and base 60
/// `[1-9][0-9_]*(:[0-5]?[0-9])+`, unsigned; `Some(None)` on overflow.
fn yaml_1_1_int(s: &str) -> Option<Option<i128>> {
    let clean = |digits: &str| digits.replace('_', "");
    if let Some(binary) = s.strip_prefix("0b") {
        return digits_or_underscores(binary, 2)
            .then(|| i128::from_str_radix(&clean(binary), 2).ok());
    }
    if let Some(hex) = s.strip_prefix("0x") {
        return digits_or_underscores(hex, 16).then(|| i128::from_str_radix(&clean(hex), 16).ok());
    }
    if s == "0" {
        return Some(Some(0));
    }
    if let Some(octal) = s.strip_prefix('0') {
        return digits_or_underscores(octal, 8)
            .then(|| i128::from_str_radix(&clean(octal), 8).ok());
    }
    if s.starts_with(|c: char| ('1'..='9').contains(&c)) {
        if digits_or_underscores(s, 10) {
            return Some(clean(s).parse::<i128>().ok());
        }
        if let Some(parts) = base_60_parts(s) {
            let (first, rest) = parts.split_first()?;
            let mut value = clean(first).parse::<i128>().ok();
            for part in rest {
                value = value
                    .and_then(|v| v.checked_mul(60))
                    .and_then(|v| v.checked_add(i128::from(part.parse::<u8>().ok()?)));
            }
            return Some(value);
        }
    }
    None
}

/// YAML 1.1 floats in the form parsers implement (PyYAML's resolver):
/// `[-+]?[0-9][0-9_]*\.[0-9_]*([eE][-+][0-9]+)?`, `\.[0-9][0-9_]*([eE][-+][0-9]+)?`
/// and base 60 `[-+]?[0-9][0-9_]*(:[0-5]?[0-9])+\.[0-9_]*`.
fn yaml_1_1_float(text: &str, unsigned: &str, negative: bool) -> Option<Option<f64>> {
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
        None => (unsigned, None),
    };
    let (whole, fraction) = mantissa.split_once('.')?;
    if fraction.is_empty() || digits_or_underscores(fraction, 10) {
        if exponent.is_none()
            && let Some(parts) = base_60_parts(whole)
        {
            let mut value = 0f64;
            for part in &parts {
                value = value * 60.0 + part.replace('_', "").parse::<f64>().ok()?;
            }
            let fraction = format!("0.{}", fraction.replace('_', ""));
            let value = value + fraction.parse::<f64>().unwrap_or(0.0);
            return Some(Some(if negative { -value } else { value }));
        }
    } else {
        return None;
    }
    let exponent_ok = exponent.is_none_or(|e| {
        e.strip_prefix(['-', '+'])
            .is_some_and(|digits| is_digits(digits, 10))
    });
    let form_ok = if whole.is_empty() {
        // The leading-dot form takes no sign and needs a digit first.
        unsigned == text && fraction.starts_with(|c: char| c.is_ascii_digit())
    } else {
        whole.starts_with(|c: char| c.is_ascii_digit()) && digits_or_underscores(whole, 10)
    };
    if !(form_ok && exponent_ok) {
        return None;
    }
    Some(text.replace('_', "").parse::<f64>().ok())
}

/// The `:`-separated parts of a base-60 number when every part after the first
/// is one or two digits below 60 and the first is digits and underscores.
fn base_60_parts(s: &str) -> Option<Vec<&str>> {
    let parts: Vec<&str> = s.split(':').collect();
    let (first, rest) = parts.split_first()?;
    let ok = !rest.is_empty()
        && first.starts_with(|c: char| c.is_ascii_digit())
        && digits_or_underscores(first, 10)
        && rest.iter().all(|part| {
            (1..=2).contains(&part.len())
                && is_digits(part, 10)
                && part.parse::<u8>().is_ok_and(|n| n < 60)
        });
    ok.then_some(parts)
}

fn signed(value: Option<i128>, negative: bool) -> Option<i128> {
    if negative { value.map(|v| -v) } else { value }
}

/// Non-empty and every byte a digit of `radix`.
fn is_digits(s: &str, radix: u32) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_digit(radix))
}

/// Non-empty and every byte a digit of `radix` or `_`.
fn digits_or_underscores(s: &str, radix: u32) -> bool {
    !s.is_empty() && s.chars().all(|c| c == '_' || c.is_digit(radix))
}

/// `(\.[0-9]+ | [0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?` that is not a plain integer.
fn is_decimal_float(s: &str) -> bool {
    let (mantissa, exponent) = match s.find(['e', 'E']) {
        Some(at) => (&s[..at], Some(&s[at + 1..])),
        None => (s, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some(("", fraction)) => is_digits(fraction, 10),
        Some((whole, fraction)) => {
            is_digits(whole, 10) && (fraction.is_empty() || is_digits(fraction, 10))
        }
        None => is_digits(mantissa, 10),
    };
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['-', '+']).unwrap_or(e);
        is_digits(e, 10)
    });
    mantissa_ok && exponent_ok && (mantissa.contains('.') || exponent.is_some())
}

/// YAML 1.1 timestamps: `2024-01-01` and `2024-1-1 10:00:00.5 +1`.
fn is_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    let digits = |from: usize, min: usize, max: usize| {
        let n = b[from.min(b.len())..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        (min..=max).contains(&n).then_some(from + n)
    };
    let at = |i: usize, c: u8| b.get(i) == Some(&c);
    let Some(i) = digits(0, 4, 4).filter(|&i| at(i, b'-')) else {
        return false;
    };
    let Some(i) = digits(i + 1, 1, 2).filter(|&i| at(i, b'-')) else {
        return false;
    };
    let Some(i) = digits(i + 1, 1, 2) else {
        return false;
    };
    if i == b.len() {
        return b.len() == 10;
    }
    let mut i = i;
    if matches!(b[i], b'T' | b't') {
        i += 1;
    } else {
        let spaces = b[i..]
            .iter()
            .take_while(|c| matches!(c, b' ' | b'\t'))
            .count();
        if spaces == 0 {
            return false;
        }
        i += spaces;
    }
    let Some(i) = digits(i, 1, 2).filter(|&i| at(i, b':')) else {
        return false;
    };
    let Some(i) = digits(i + 1, 2, 2).filter(|&i| at(i, b':')) else {
        return false;
    };
    let Some(mut i) = digits(i + 1, 2, 2) else {
        return false;
    };
    if at(i, b'.') {
        i += 1 + b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
    }
    let zone = &s[i..];
    let zone = zone.trim_start_matches([' ', '\t']);
    match zone.as_bytes().first() {
        None => true,
        Some(b'Z') => zone.len() == 1,
        Some(b'-' | b'+') => {
            let offset = &zone[1..];
            let (hours, minutes) = offset.split_once(':').unwrap_or((offset, "00"));
            (1..=2).contains(&hours.len())
                && minutes.len() == 2
                && hours
                    .bytes()
                    .chain(minutes.bytes())
                    .all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}
