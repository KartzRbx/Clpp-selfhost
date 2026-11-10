#[path = "parser.rs"]
pub mod parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    DecimalInteger,
    BinaryInteger,
    HexInteger,
    Float,
    String,
    Symbol,
    EndOfFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: &'static str,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    source: &'a str,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn scan_tokens(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_trivia()?;
            if self.position == self.source.len() {
                tokens.push(Token {
                    kind: TokenKind::EndOfFile,
                    start: self.position,
                    end: self.position,
                    line: self.line,
                    column: self.column,
                });
                return Ok(tokens);
            }

            tokens.push(self.scan_token()?);
        }
    }

    fn scan_token(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        let line = self.line;
        let column = self.column;

        match self.peek().expect("position is before end of source") {
            ch if is_identifier_start(ch) => {
                self.advance();
                while matches!(self.peek(), Some(ch) if is_identifier_part(ch)) {
                    self.advance();
                }
                Ok(self.token(TokenKind::Identifier, start, line, column))
            }
            ch if ch.is_ascii_digit() => self.scan_number(start, line, column),
            '"' | '\'' => self.scan_string(start, line, column),
            _ => self.scan_symbol(start, line, column),
        }
    }

    fn scan_number(&mut self, start: usize, line: usize, column: usize) -> Result<Token, LexError> {
        let (kind, radix) = if self.source[self.position..].starts_with("0x")
            || self.source[self.position..].starts_with("0X")
        {
            (TokenKind::HexInteger, 16)
        } else if self.source[self.position..].starts_with("0b")
            || self.source[self.position..].starts_with("0B")
        {
            (TokenKind::BinaryInteger, 2)
        } else {
            (TokenKind::DecimalInteger, 10)
        };

        if radix != 10 {
            self.advance();
            self.advance();
        }

        let mut digit_count = 0;
        let mut previous_was_digit = false;

        while let Some(ch) = self.peek() {
            let valid_digit = match radix {
                2 => ch == '0' || ch == '1',
                10 => ch.is_ascii_digit(),
                _ => ch.is_ascii_hexdigit(),
            };

            if valid_digit {
                digit_count += 1;
                previous_was_digit = true;
                self.advance();
            } else if ch == '_' {
                if !previous_was_digit {
                    return Err(self.error("invalid digit separator"));
                }
                previous_was_digit = false;
                self.advance();
            } else {
                break;
            }
        }

        if digit_count == 0 || !previous_was_digit {
            return Err(self.error("invalid numeric literal"));
        }

        let mut token_kind = kind;
        if radix == 10 {
            if self.peek() == Some('.') && !self.source[self.position..].starts_with("..") {
                let fractional_start = self.position + 1;
                if matches!(
                    self.source[fractional_start..].chars().next(),
                    Some(ch) if ch.is_ascii_digit()
                ) {
                    token_kind = TokenKind::Float;
                    self.advance();
                    self.scan_decimal_component()?;
                }
            }

            if matches!(self.peek(), Some('e' | 'E')) {
                token_kind = TokenKind::Float;
                self.advance();
                if matches!(self.peek(), Some('+' | '-')) {
                    self.advance();
                }
                self.scan_decimal_component()?;
            }
        }

        if matches!(self.peek(), Some(ch) if is_identifier_part(ch)) {
            return Err(self.error("invalid character after numeric literal"));
        }

        Ok(self.token(token_kind, start, line, column))
    }

    fn scan_decimal_component(&mut self) -> Result<(), LexError> {
        let mut digit_count = 0;
        let mut previous_was_digit = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                digit_count += 1;
                previous_was_digit = true;
                self.advance();
            } else if ch == '_' {
                if !previous_was_digit {
                    return Err(self.error("invalid digit separator"));
                }
                previous_was_digit = false;
                self.advance();
            } else {
                break;
            }
        }

        if digit_count == 0 || !previous_was_digit {
            return Err(self.error("invalid numeric literal"));
        }

        Ok(())
    }

    fn scan_string(&mut self, start: usize, line: usize, column: usize) -> Result<Token, LexError> {
        let quote = self.advance().expect("string begins with a quote");

        loop {
            match self.peek() {
                Some(ch) if ch == quote => {
                    self.advance();
                    return Ok(self.token(TokenKind::String, start, line, column));
                }
                Some('\n' | '\r') => {
                    return Err(self.error("newline in quoted string"));
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n' | 't' | '\\' | '"' | '\'') => {}
                        Some(_) => return Err(self.error("invalid escape sequence")),
                        None => return Err(self.error("unterminated escape sequence")),
                    }
                }
                Some(_) => {
                    self.advance();
                }
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn scan_symbol(&mut self, start: usize, line: usize, column: usize) -> Result<Token, LexError> {
        const MULTI_CHAR_SYMBOLS: &[&str] = &[
            "...", "::", ".:", "..", "=>", "->", "~>", "==", "!=", "<=", ">=", "+=", "-=", "*=",
            "/=", "%=", "++", "--",
        ];
        const SINGLE_CHAR_SYMBOLS: &str = "+-*/%=<>!&|^~?()[]{},;.:@";

        if let Some(symbol) = MULTI_CHAR_SYMBOLS
            .iter()
            .find(|symbol| self.source[self.position..].starts_with(**symbol))
        {
            for _ in symbol.chars() {
                self.advance();
            }
            return Ok(self.token(TokenKind::Symbol, start, line, column));
        }

        let ch = self.peek().expect("position is before end of source");
        if SINGLE_CHAR_SYMBOLS.contains(ch) {
            self.advance();
            return Ok(self.token(TokenKind::Symbol, start, line, column));
        }

        Err(self.error("unexpected character"))
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\n' | '\r') => {
                    self.advance();
                }
                Some(_) if self.source[self.position..].starts_with("<<[") => {
                    let start_line = self.line;
                    let start_column = self.column;
                    self.advance();
                    self.advance();
                    self.advance();

                    while !self.source[self.position..].starts_with("]>>") {
                        if self.advance().is_none() {
                            return Err(LexError {
                                message: "unterminated block comment",
                                line: start_line,
                                column: start_column,
                            });
                        }
                    }

                    self.advance();
                    self.advance();
                    self.advance();
                }
                Some(_) if self.source[self.position..].starts_with("<<") => {
                    self.advance();
                    self.advance();
                    while !matches!(self.peek(), None | Some('\n' | '\r')) {
                        self.advance();
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn token(&self, kind: TokenKind, start: usize, line: usize, column: usize) -> Token {
        Token {
            kind,
            start,
            end: self.position,
            line,
            column,
        }
    }

    fn error(&self, message: &'static str) -> LexError {
        LexError {
            message,
            line: self.line,
            column: self.column,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.position..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.position += ch.len_utf8();

        match ch {
            '\n' => {
                self.line += 1;
                self.column = 1;
            }
            '\r' if self.peek() != Some('\n') => {
                self.line += 1;
                self.column = 1;
            }
            '\r' => {}
            _ => self.column += 1,
        }

        Some(ch)
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::{Lexer, TokenKind};

    #[test]
    fn scans_official_comment_forms_and_locations() {
        let source = "<< line\nnumber x = 1; <<[ block\ncomment ]>>";
        let tokens = Lexer::new(source).scan_tokens().unwrap();

        assert_eq!(&source[tokens[0].start..tokens[0].end], "number");
        assert_eq!((tokens[0].line, tokens[0].column), (2, 1));
        assert_eq!(&source[tokens[3].start..tokens[3].end], "1");
        assert_eq!(tokens[5].kind, TokenKind::EndOfFile);
    }

    #[test]
    fn scans_longest_symbols_and_module_alias_words() {
        let source = "link \"./token.clp\" as Token; a==b..c !as";
        let tokens = Lexer::new(source).scan_tokens().unwrap();
        let lexemes: Vec<_> = tokens
            .iter()
            .map(|token| &source[token.start..token.end])
            .collect();

        assert_eq!(
            lexemes,
            vec![
                "link",
                "\"./token.clp\"",
                "as",
                "Token",
                ";",
                "a",
                "==",
                "b",
                "..",
                "c",
                "!",
                "as",
                ""
            ]
        );
    }

    #[test]
    fn tokenizes_module_naming_style_directive() {
        let source = "!as MinhaBiblioteca\n!style functions=camelCase, variables=UPPER_SNAKE_CASE";
        let tokens = Lexer::new(source).scan_tokens().unwrap();
        let lexemes: Vec<_> = tokens
            .iter()
            .map(|token| &source[token.start..token.end])
            .collect();

        assert_eq!(
            lexemes,
            vec![
                "!",
                "as",
                "MinhaBiblioteca",
                "!",
                "style",
                "functions",
                "=",
                "camelCase",
                ",",
                "variables",
                "=",
                "UPPER_SNAKE_CASE",
                ""
            ]
        );
    }

    #[test]
    fn uses_utf8_byte_offsets_for_unicode_source() {
        let source = "\"é\" name";
        let tokens = Lexer::new(source).scan_tokens().unwrap();

        assert_eq!(&source[tokens[0].start..tokens[0].end], "\"é\"");
        assert_eq!((tokens[0].start, tokens[0].end), (0, 4));
        assert_eq!(&source[tokens[1].start..tokens[1].end], "name");
        assert_eq!((tokens[1].start, tokens[1].end), (5, 9));
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let error = Lexer::new("<<[ never closed").scan_tokens().unwrap_err();
        assert_eq!(error.message, "unterminated block comment");
        assert_eq!((error.line, error.column), (1, 1));
    }

    #[test]
    fn distinguishes_integer_bases_without_reclassifying_hex_colors() {
        let tokens = Lexer::new("42 0b1010 0xFF 0xAABBCC 0xFFAABBCC")
            .scan_tokens()
            .unwrap();

        assert_eq!(tokens[0].kind, TokenKind::DecimalInteger);
        assert_eq!(tokens[1].kind, TokenKind::BinaryInteger);
        assert_eq!(tokens[2].kind, TokenKind::HexInteger);
        assert_eq!(tokens[3].kind, TokenKind::HexInteger);
        assert_eq!(tokens[4].kind, TokenKind::HexInteger);
    }

    #[test]
    fn distinguishes_decimal_floats() {
        let tokens = Lexer::new("3.5 1e3 2.5e-3").scan_tokens().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Float);
        assert_eq!(tokens[1].kind, TokenKind::Float);
        assert_eq!(tokens[2].kind, TokenKind::Float);
    }

    #[test]
    fn rejects_invalid_binary_digits() {
        let error = Lexer::new("0b102").scan_tokens().unwrap_err();
        assert_eq!(error.message, "invalid character after numeric literal");
    }
}
