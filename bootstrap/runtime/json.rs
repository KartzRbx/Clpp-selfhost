//! Minimal JSON value, parser and writer for the language server.

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj(pairs: Vec<(&str, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    pub fn num(n: impl Into<f64>) -> Json {
        Json::Num(n.into())
    }

    pub fn get(&self, key: &str) -> &Json {
        static NULL: Json = Json::Null;
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v).unwrap_or(&NULL),
            _ => &NULL,
        }
    }

    pub fn at(&self, index: usize) -> &Json {
        static NULL: Json = Json::Null;
        match self {
            Json::Arr(items) => items.get(index).unwrap_or(&NULL),
            _ => &NULL,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        match self {
            Json::Num(n) if *n >= 0.0 => Some(*n as usize),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    pub fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 9e15 {
                    let _ = write!(out, "{}", *n as i64);
                } else {
                    let _ = write!(out, "{}", n);
                }
            }
            Json::Str(s) => write_string(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(pairs) => {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
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

pub fn parse(text: &str) -> Result<Json, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut p = P { c: chars, i: 0 };
    let v = p.value()?;
    p.ws();
    if p.i != p.c.len() {
        return Err(format!("trailing characters at {}", p.i));
    }
    Ok(v)
}

struct P {
    c: Vec<char>,
    i: usize,
}

impl P {
    fn ws(&mut self) {
        while self.i < self.c.len() && self.c[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, ch: char) -> bool {
        self.ws();
        if self.i < self.c.len() && self.c[self.i] == ch {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn lit(&mut self, word: &str, v: Json) -> Result<Json, String> {
        for ch in word.chars() {
            if self.i >= self.c.len() || self.c[self.i] != ch {
                return Err(format!("invalid literal at {}", self.i));
            }
            self.i += 1;
        }
        Ok(v)
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        let ch = *self.c.get(self.i).ok_or("unexpected end of JSON")?;
        match ch {
            'n' => self.lit("null", Json::Null),
            't' => self.lit("true", Json::Bool(true)),
            'f' => self.lit("false", Json::Bool(false)),
            '"' => Ok(Json::Str(self.string()?)),
            '[' => {
                self.i += 1;
                let mut items = Vec::new();
                if self.eat(']') {
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    if self.eat(',') {
                        continue;
                    }
                    if self.eat(']') {
                        return Ok(Json::Arr(items));
                    }
                    return Err(format!("expected ',' or ']' at {}", self.i));
                }
            }
            '{' => {
                self.i += 1;
                let mut pairs = Vec::new();
                if self.eat('}') {
                    return Ok(Json::Obj(pairs));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    if !self.eat(':') {
                        return Err(format!("expected ':' at {}", self.i));
                    }
                    pairs.push((key, self.value()?));
                    if self.eat(',') {
                        continue;
                    }
                    if self.eat('}') {
                        return Ok(Json::Obj(pairs));
                    }
                    return Err(format!("expected ',' or '}}' at {}", self.i));
                }
            }
            _ => {
                let start = self.i;
                while self.i < self.c.len() && matches!(self.c[self.i], '-' | '+' | '.' | 'e' | 'E' | '0'..='9') {
                    self.i += 1;
                }
                let text: String = self.c[start..self.i].iter().collect();
                text.parse::<f64>().map(Json::Num).map_err(|_| format!("invalid number '{}'", text))
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.c.get(self.i) != Some(&'"') {
            return Err(format!("expected string at {}", self.i));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let ch = *self.c.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match ch {
                '"' => return Ok(out),
                '\\' => {
                    let e = *self.c.get(self.i).ok_or("unterminated escape")?;
                    self.i += 1;
                    match e {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let hex: String = self.c.get(self.i..self.i + 4).ok_or("bad \\u escape")?.iter().collect();
                            self.i += 4;
                            let mut code = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u escape")?;
                            if (0xD800..0xDC00).contains(&code) && self.c.get(self.i) == Some(&'\\') {
                                let low: String = self.c.get(self.i + 2..self.i + 6).ok_or("bad surrogate")?.iter().collect();
                                if let Ok(low) = u32::from_str_radix(&low, 16) {
                                    self.i += 6;
                                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                }
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
    }
}
