//! Just enough JSON for the machine protocol (`frus daemon`, `frus run --machine`): a value,
//! read from a line and written to one. The tool has no dependencies, and what an editor sends
//! it is small: an id, a method, a few parameters.

use std::fmt::Write as _;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    /// Members in the order they were written.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member `key` of an object.
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The value on one line, as JSON.
    pub(crate) fn to_json(&self) -> String {
        let mut out = String::new();
        write_value(self, &mut out);
        out
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::String(s)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Bool(b)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Value {
        Value::Number(n)
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Value {
        Value::Number(n as f64)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Value {
        v.map_or(Value::Null, Into::into)
    }
}

/// An object from `(key, value)` pairs: `object([("id", 1.0.into())])`.
pub(crate) fn object<const N: usize>(members: [(&str, Value); N]) -> Value {
    Value::Object(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn write_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => {
            let _ = write!(out, "{}", *n as i64);
        }
        Value::Number(n) if n.is_finite() => {
            let _ = write!(out, "{n}");
        }
        Value::Number(_) => out.push_str("null"),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (i, (key, item)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_value(item, out);
            }
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Reads one JSON value; text after it, other than whitespace, is an error.
pub(crate) fn parse(text: &str) -> Result<Value, String> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        at: 0,
    };
    let value = parser.value()?;
    parser.space();
    if parser.at != parser.chars.len() {
        return Err(format!("unexpected text at {}", parser.at));
    }
    Ok(value)
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        self.space();
        if self.peek() == Some(c) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("expected `{c}` at {}", self.at))
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        let end = self.at + word.chars().count();
        if self
            .chars
            .get(self.at..end)
            .is_some_and(|w| w.iter().copied().eq(word.chars()))
        {
            self.at = end;
            Ok(value)
        } else {
            Err(format!("unexpected text at {}", self.at))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.space();
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => self.string().map(Value::String),
            Some('t') => self.literal("true", Value::Bool(true)),
            Some('f') => self.literal("false", Value::Bool(false)),
            Some('n') => self.literal("null", Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            _ => Err(format!("unexpected text at {}", self.at)),
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.expect('{')?;
        let mut members = Vec::new();
        self.space();
        if self.peek() == Some('}') {
            self.at += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.space();
            let key = self.string()?;
            self.expect(':')?;
            members.push((key, self.value()?));
            self.space();
            match self.peek() {
                Some(',') => self.at += 1,
                Some('}') => {
                    self.at += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(format!("expected `,` or `}}` at {}", self.at)),
            }
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(']') {
            self.at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.space();
            match self.peek() {
                Some(',') => self.at += 1,
                Some(']') => {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(format!("expected `,` or `]` at {}", self.at)),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.peek() != Some('"') {
            return Err(format!("expected a string at {}", self.at));
        }
        self.at += 1;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or("unterminated string")?;
            self.at += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let escaped = self.peek().ok_or("unterminated string")?;
                    self.at += 1;
                    match escaped {
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let hex: String = self
                                .chars
                                .get(self.at..self.at + 4)
                                .ok_or("bad escape")?
                                .iter()
                                .collect();
                            self.at += 4;
                            let code = u32::from_str_radix(&hex, 16).map_err(|_| "bad escape")?;
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || "+-.eE".contains(c))
        {
            self.at += 1;
        }
        let text: String = self.chars[start..self.at].iter().collect();
        text.parse()
            .map(Value::Number)
            .map_err(|_| format!("bad number `{text}`"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_is_read() {
        let command =
            parse(r#"{"id": 3, "method": "app.restart", "params": {"fullRestart": true, "x": [1, null, "a\"b\n"]}}"#)
                .unwrap();
        assert_eq!(command.get("id"), Some(&Value::Number(3.0)));
        assert_eq!(
            command.get("method").and_then(Value::as_str),
            Some("app.restart")
        );
        let params = command.get("params").unwrap();
        assert_eq!(
            params.get("fullRestart").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            params.get("x"),
            Some(&Value::Array(vec![
                Value::Number(1.0),
                Value::Null,
                Value::String("a\"b\n".into())
            ]))
        );
    }

    #[test]
    fn values_are_written_on_one_line_and_read_back() {
        let value = object([
            ("event", "app.log".into()),
            (
                "params",
                object([
                    ("log", "tab\there \"quoted\"\nnext".into()),
                    ("error", false.into()),
                ]),
            ),
            ("n", 1.5.into()),
            ("id", 7u64.into()),
            ("none", Value::Null),
        ]);
        let line = value.to_json();
        assert!(!line.contains('\n'), "{line}");
        assert_eq!(parse(&line).unwrap(), value);
        assert!(line.contains(r#""id":7"#), "{line}");
    }

    #[test]
    fn broken_text_is_refused() {
        for text in ["", "{", r#"{"a" 1}"#, "[1,]", "nul", r#""open"#, "{} x"] {
            assert!(parse(text).is_err(), "{text}");
        }
    }
}
