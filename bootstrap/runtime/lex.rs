//! Tokenizer for the stage-0 interpreter.
//!
//! Mirrors `src/lexer/lexer.clp`: `<<` line comments, `<<[ ... ]>>` block
//! comments, quoted strings with escapes, decimal/hex/binary/float numbers
//! with `_` separators, and the CL++ symbol table (longest match first).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tk {
    Ident,
    Int,
    Float,
    Str,
    /// `` `text ${expr}` ``: the raw text between the backticks.
    Template,
    Sym,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Tk,
    /// Decoded text (string literals without quotes and escapes).
    pub text: String,
    pub line: usize,
    pub column: usize,
    /// Character offsets of the original lexeme in the source (end exclusive).
    pub start: usize,
    pub end: usize,
}

/// A `<<`, `<<[ ]>>` or `<<{ }>>` comment, kept for the formatter.
#[derive(Debug, Clone)]
pub struct Comment {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub end_line: usize,
}

const SYMBOLS: &[&str] = &[
    "...", "::", ".:", "..", "=>", "->", "~>", "==", "!=", "<=", ">=", "+=", "-=", "*=", "/=",
    "%=", "++", "--", "+", "-", "*", "/", "%", "=", "<", ">", "!", "&", "|", "^", "~", "?", ".",
    ":", "@", "(", ")", "[", "]", "{", "}", ",", ";",
];

pub fn lex(source: &str, file: &str) -> Result<Vec<Token>, String> {
    lex_full(source, file).map(|(tokens, _)| tokens)
}

pub fn lex_full(source: &str, file: &str) -> Result<(Vec<Token>, Vec<Comment>), String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut comments = Vec::new();
    let mut pos = 0usize;
    let mut line = 1usize;
    let mut column = 1usize;

    macro_rules! err {
        ($l:expr, $c:expr, $($arg:tt)*) => {
            return Err(format!("{}:{}:{}: {}", file, $l, $c, format!($($arg)*)))
        };
    }

    let starts = |pos: usize, text: &str| -> bool {
        let mut i = pos;
        for ch in text.chars() {
            if i >= chars.len() || chars[i] != ch {
                return false;
            }
            i += 1;
        }
        true
    };

    while pos < chars.len() {
        let ch = chars[pos];
        // Trivia.
        if ch == '\n' {
            pos += 1;
            line += 1;
            column = 1;
            continue;
        }
        if ch == '\r' {
            pos += 1;
            if pos >= chars.len() || chars[pos] != '\n' {
                line += 1;
                column = 1;
            }
            continue;
        }
        if ch == ' ' || ch == '\t' || ch == '\u{feff}' {
            pos += 1;
            column += 1;
            continue;
        }
        if starts(pos, "<<{") {
            // Doc comments only matter to the self-hosted parser and tooling.
            let (start_line, start_column) = (line, column);
            let comment_start = pos;
            pos += 3;
            column += 3;
            loop {
                if pos >= chars.len() {
                    err!(start_line, start_column, "unterminated doc comment");
                }
                if starts(pos, "}>>") {
                    pos += 3;
                    column += 3;
                    comments.push(Comment { start: comment_start, end: pos, line: start_line, end_line: line });
                    break;
                }
                if chars[pos] == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
                pos += 1;
            }
            continue;
        }
        if starts(pos, "<<[") {
            let (start_line, start_column) = (line, column);
            let comment_start = pos;
            pos += 3;
            column += 3;
            loop {
                if pos >= chars.len() {
                    err!(start_line, start_column, "unterminated block comment");
                }
                if starts(pos, "]>>") {
                    pos += 3;
                    column += 3;
                    comments.push(Comment { start: comment_start, end: pos, line: start_line, end_line: line });
                    break;
                }
                if chars[pos] == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
                pos += 1;
            }
            continue;
        }
        if starts(pos, "<<") {
            let comment_start = pos;
            while pos < chars.len() && chars[pos] != '\n' && chars[pos] != '\r' {
                pos += 1;
                column += 1;
            }
            comments.push(Comment { start: comment_start, end: pos, line, end_line: line });
            continue;
        }

        let start = pos;
        let (start_line, start_column) = (line, column);

        if ch.is_ascii_alphabetic() || ch == '_' {
            while pos < chars.len() && (chars[pos].is_ascii_alphanumeric() || chars[pos] == '_') {
                pos += 1;
            }
            column += pos - start;
            tokens.push(Token {
                kind: Tk::Ident,
                text: chars[start..pos].iter().collect(),
                line: start_line,
                column: start_column,
                start,
                end: pos,
            });
            continue;
        }

        if ch.is_ascii_digit() {
            let mut kind = Tk::Int;
            let radix_prefix = ch == '0'
                && pos + 1 < chars.len()
                && matches!(chars[pos + 1], 'x' | 'X' | 'b' | 'B');
            if radix_prefix {
                let hex = matches!(chars[pos + 1], 'x' | 'X');
                pos += 2;
                while pos < chars.len()
                    && (chars[pos] == '_'
                        || (hex && chars[pos].is_ascii_hexdigit())
                        || (!hex && (chars[pos] == '0' || chars[pos] == '1')))
                {
                    pos += 1;
                }
            } else {
                while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '_') {
                    pos += 1;
                }
                if pos + 1 < chars.len() && chars[pos] == '.' && chars[pos + 1].is_ascii_digit() {
                    kind = Tk::Float;
                    pos += 1;
                    while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == '_') {
                        pos += 1;
                    }
                }
                if pos < chars.len() && (chars[pos] == 'e' || chars[pos] == 'E') {
                    kind = Tk::Float;
                    pos += 1;
                    if pos < chars.len() && (chars[pos] == '+' || chars[pos] == '-') {
                        pos += 1;
                    }
                    while pos < chars.len() && chars[pos].is_ascii_digit() {
                        pos += 1;
                    }
                }
            }
            if pos < chars.len() && (chars[pos].is_ascii_alphanumeric() || chars[pos] == '_') {
                err!(line, column + (pos - start), "invalid character after numeric literal");
            }
            column += pos - start;
            tokens.push(Token {
                kind,
                text: chars[start..pos].iter().collect(),
                line: start_line,
                column: start_column,
                start,
                end: pos,
            });
            continue;
        }

        if ch == '"' || ch == '\'' {
            let quote = ch;
            pos += 1;
            let mut value = String::new();
            loop {
                if pos >= chars.len() {
                    err!(start_line, start_column, "unterminated string");
                }
                let c = chars[pos];
                if c == quote {
                    pos += 1;
                    break;
                }
                if c == '\n' || c == '\r' {
                    err!(start_line, start_column, "newline in quoted string");
                }
                if c == '\\' {
                    pos += 1;
                    if pos >= chars.len() {
                        err!(start_line, start_column, "unterminated escape sequence");
                    }
                    match chars[pos] {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '0' => value.push('\0'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        '\'' => value.push('\''),
                        other => err!(line, column + (pos - start), "invalid escape sequence '\\{}'", other),
                    }
                    pos += 1;
                    continue;
                }
                value.push(c);
                pos += 1;
            }
            column += pos - start;
            tokens.push(Token { kind: Tk::Str, text: value, line: start_line, column: start_column, start, end: pos });
            continue;
        }

        if ch == '`' {
            // Raw content up to the closing backtick; `${...}` may nest braces,
            // strings and other templates. The parser splits it.
            let (start_line, start_column) = (line, column);
            pos += 1;
            column += 1;
            let content_start = pos;
            let mut depth = 0usize;
            loop {
                if pos >= chars.len() {
                    err!(start_line, start_column, "unterminated template string");
                }
                let c = chars[pos];
                if c == '\\' && pos + 1 < chars.len() {
                    pos += 2;
                    column += 2;
                    continue;
                }
                if depth == 0 && c == '`' {
                    break;
                }
                if c == '$' && pos + 1 < chars.len() && chars[pos + 1] == '{' {
                    depth += 1;
                    pos += 2;
                    column += 2;
                    continue;
                }
                if depth > 0 && c == '{' {
                    depth += 1;
                } else if depth > 0 && c == '}' {
                    depth -= 1;
                }
                if c == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
                pos += 1;
            }
            let content: String = chars[content_start..pos].iter().collect();
            pos += 1;
            column += 1;
            tokens.push(Token { kind: Tk::Template, text: content, line: start_line, column: start_column, start, end: pos });
            continue;
        }

        let mut matched = None;
        for symbol in SYMBOLS {
            if starts(pos, symbol) {
                matched = Some(*symbol);
                break;
            }
        }
        match matched {
            Some(symbol) => {
                let width = symbol.chars().count();
                pos += width;
                column += width;
                tokens.push(Token {
                    kind: Tk::Sym,
                    text: symbol.to_string(),
                    line: start_line,
                    column: start_column,
                    start,
                    end: pos,
                });
            }
            None => err!(line, column, "unexpected character '{}'", ch),
        }
    }

    tokens.push(Token { kind: Tk::Eof, text: String::new(), line, column, start: pos, end: pos });
    Ok((tokens, comments))
}
