use super::{Lexer, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingTarget {
    Functions,
    Variables,
    Parameters,
}

impl NamingTarget {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "functions" => Some(Self::Functions),
            "variables" => Some(Self::Variables),
            "parameters" => Some(Self::Parameters),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Functions => "functions",
            Self::Variables => "variables",
            Self::Parameters => "parameters",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingConvention {
    Uppercase,
    CamelCase,
    PascalCase,
    UpperSnakeCase,
}

impl NamingConvention {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "UPPERCASE" => Some(Self::Uppercase),
            "camelCase" => Some(Self::CamelCase),
            "PascalCase" => Some(Self::PascalCase),
            "UPPER_SNAKE_CASE" => Some(Self::UpperSnakeCase),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uppercase => "UPPERCASE",
            Self::CamelCase => "camelCase",
            Self::PascalCase => "PascalCase",
            Self::UpperSnakeCase => "UPPER_SNAKE_CASE",
        }
    }

    pub fn matches(self, name: &str) -> bool {
        if name.is_empty() || !name.is_ascii() {
            return false;
        }

        match self {
            Self::Uppercase => {
                name.as_bytes()[0].is_ascii_uppercase()
                    && name
                        .bytes()
                        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
            }
            Self::CamelCase => {
                name.as_bytes()[0].is_ascii_lowercase()
                    && name.bytes().all(|ch| ch.is_ascii_alphanumeric())
            }
            Self::PascalCase => {
                name.as_bytes()[0].is_ascii_uppercase()
                    && name.bytes().all(|ch| ch.is_ascii_alphanumeric())
            }
            Self::UpperSnakeCase => {
                name.as_bytes()[0].is_ascii_uppercase()
                    && name.split('_').all(|part| {
                        !part.is_empty()
                            && part
                                .bytes()
                                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
                    })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamingRule {
    pub target: NamingTarget,
    pub convention: NamingConvention,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleHeader {
    pub name: String,
    pub naming_rules: Vec<NamingRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModule {
    pub header: ModuleHeader,
    pub type_definitions: Vec<TypeDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    pub name: String,
    pub value: TypeExpression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpression {
    Named {
        name: String,
        arguments: Vec<TypeExpression>,
    },
    Intersection(Vec<TypeExpression>),
    Object(Vec<TypeField>),
    Typeof(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeField {
    pub name: String,
    pub value: TypeExpression,
}

impl ModuleHeader {
    pub fn convention_for(&self, target: NamingTarget) -> Option<NamingConvention> {
        self.naming_rules
            .iter()
            .find(|rule| rule.target == target)
            .map(|rule| rule.convention)
    }

    pub fn naming_warning(&self, target: NamingTarget, name: &str) -> Option<String> {
        let convention = self.convention_for(target)?;
        if convention.matches(name) {
            return None;
        }

        Some(format!(
            "naming style warning: {} name '{}' should use {}",
            target.as_str(),
            name,
            convention.as_str()
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: &'static str,
    pub line: usize,
    pub column: usize,
}

struct HeaderParser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    current: usize,
}

impl<'a> HeaderParser<'a> {
    fn new(source: &'a str) -> Result<Self, ParseError> {
        let tokens = Lexer::new(source)
            .scan_tokens()
            .map_err(|error| ParseError {
                message: error.message,
                line: error.line,
                column: error.column,
            })?;
        Ok(Self {
            source,
            tokens,
            current: 0,
        })
    }

    fn token(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn text(&self, index: usize) -> &str {
        let token = &self.tokens[index];
        &self.source[token.start..token.end]
    }

    fn check_text(&self, text: &str) -> bool {
        self.token().kind != TokenKind::EndOfFile && self.text(self.current) == text
    }

    fn match_text(&mut self, text: &str) -> bool {
        if !self.check_text(text) {
            return false;
        }
        self.current += 1;
        true
    }

    fn error(&self, message: &'static str) -> ParseError {
        ParseError {
            message,
            line: self.token().line,
            column: self.token().column,
        }
    }

    fn expect_text(&mut self, text: &str, message: &'static str) -> Result<(), ParseError> {
        if self.match_text(text) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn expect_identifier(&mut self, message: &'static str) -> Result<String, ParseError> {
        if self.token().kind != TokenKind::Identifier {
            return Err(self.error(message));
        }
        let text = self.text(self.current).to_owned();
        self.current += 1;
        Ok(text)
    }

    fn is_at_end(&self) -> bool {
        self.token().kind == TokenKind::EndOfFile
    }

    fn parse_style_directive(&mut self, rules: &mut Vec<NamingRule>) -> Result<(), ParseError> {
        self.expect_text("!", "expected '!' before style directive")?;
        self.expect_text("style", "expected 'style' after '!'")?;

        loop {
            let target_text = self.expect_identifier("expected naming target")?;
            let target = NamingTarget::parse(&target_text)
                .ok_or_else(|| self.error("unknown naming target"))?;
            self.expect_text("=", "expected '=' after naming target")?;
            let convention_text = self.expect_identifier("expected naming convention")?;
            let convention = NamingConvention::parse(&convention_text)
                .ok_or_else(|| self.error("unknown naming convention"))?;

            if rules.iter().any(|rule| rule.target == target) {
                return Err(self.error("duplicate naming target"));
            }
            rules.push(NamingRule { target, convention });

            if !self.match_text(",") {
                break;
            }
        }

        self.match_text(";");
        Ok(())
    }

    fn parse_header(&mut self) -> Result<ModuleHeader, ParseError> {
        self.expect_text("!", "expected module header '!as Name'")?;
        self.expect_text("as", "expected 'as' after '!'")?;
        let name = self.expect_identifier("expected module name")?;
        self.match_text(";");

        let mut naming_rules = Vec::new();
        while self.check_text("!")
            && self.current + 1 < self.tokens.len()
            && self.text(self.current + 1) == "style"
        {
            self.parse_style_directive(&mut naming_rules)?;
        }

        Ok(ModuleHeader { name, naming_rules })
    }

    fn parse_type_expression(&mut self) -> Result<TypeExpression, ParseError> {
        let mut parts = vec![self.parse_type_primary()?];
        while self.match_text("&") {
            parts.push(self.parse_type_primary()?);
        }
        if parts.len() == 1 {
            Ok(parts.remove(0))
        } else {
            Ok(TypeExpression::Intersection(parts))
        }
    }

    fn parse_type_primary(&mut self) -> Result<TypeExpression, ParseError> {
        if self.match_text("typeof") {
            self.expect_text("(", "expected '(' after typeof")?;
            let mut depth = 1;
            let mut expression = Vec::new();
            while depth > 0 {
                if self.is_at_end() {
                    return Err(self.error("unterminated typeof expression"));
                }
                let text = self.text(self.current).to_owned();
                if text == "(" {
                    depth += 1;
                } else if text == ")" {
                    depth -= 1;
                    if depth == 0 {
                        self.current += 1;
                        break;
                    }
                }
                expression.push(text);
                self.current += 1;
            }
            return Ok(TypeExpression::Typeof(expression));
        }

        if self.match_text("{") {
            let mut fields = Vec::new();
            while !self.check_text("}") && !self.is_at_end() {
                let line = self.token().line;
                let name = self.expect_identifier("expected field name in object type")?;
                self.expect_text(":", "expected ':' after object type field")?;
                let value = self.parse_type_expression()?;
                fields.push(TypeField { name, value });

                if self.match_text(",") || self.match_text(";") {
                    continue;
                }
                if self.check_text("}") {
                    break;
                }
                if self.is_at_end() || self.token().line > line {
                    continue;
                }
                return Err(self.error("expected ',' or line break after object type field"));
            }
            self.expect_text("}", "expected '}' after object type")?;
            return Ok(TypeExpression::Object(fields));
        }

        let name = self.expect_identifier("expected type name")?;
        let mut arguments = Vec::new();
        if self.match_text("<") {
            loop {
                arguments.push(self.parse_type_expression()?);
                if !self.match_text(",") {
                    break;
                }
            }
            self.expect_text(">", "expected '>' after type arguments")?;
        }
        Ok(TypeExpression::Named { name, arguments })
    }

    fn parse_type_definition(&mut self) -> Result<TypeDefinition, ParseError> {
        self.match_text("export");
        self.expect_text("def", "expected 'def' before type name")?;
        let name = self.expect_identifier("expected type name after 'def'")?;
        self.expect_text("=", "expected '=' after type name")?;
        let value = self.parse_type_expression()?;

        if !self.match_text(";")
            && !self.is_at_end()
            && self.token().line == self.tokens[self.current - 1].line
        {
            return Err(self.error("expected ';' or line break after type declaration"));
        }
        Ok(TypeDefinition { name, value })
    }

    fn parse_module(mut self) -> Result<ParsedModule, ParseError> {
        let header = self.parse_header()?;
        let mut type_definitions = Vec::new();
        while !self.is_at_end() {
            type_definitions.push(self.parse_type_definition()?);
        }
        Ok(ParsedModule {
            header,
            type_definitions,
        })
    }
}

pub fn parse_module_header(source: &str) -> Result<ModuleHeader, ParseError> {
    HeaderParser::new(source)?.parse_header()
}

pub fn parse_module(source: &str) -> Result<ParsedModule, ParseError> {
    HeaderParser::new(source)?.parse_module()
}

#[cfg(test)]
mod tests {
    use super::{
        parse_module, parse_module_header, NamingConvention, NamingTarget, TypeExpression,
    };

    #[test]
    fn parses_style_rules_in_module_header() {
        let header = parse_module_header(
            "!as MinhaBiblioteca\n!style functions=camelCase, variables=UPPER_SNAKE_CASE, parameters=PascalCase",
        )
        .unwrap();

        assert_eq!(header.name, "MinhaBiblioteca");
        assert_eq!(
            header.convention_for(NamingTarget::Functions),
            Some(NamingConvention::CamelCase)
        );
        assert_eq!(
            header.convention_for(NamingTarget::Variables),
            Some(NamingConvention::UpperSnakeCase)
        );
        assert_eq!(
            header.convention_for(NamingTarget::Parameters),
            Some(NamingConvention::PascalCase)
        );
    }

    #[test]
    fn accepts_style_directive_with_optional_semicolons() {
        let header = parse_module_header(
            "!as Naming;\n!style functions=UPPERCASE;\n!style variables=camelCase;",
        )
        .unwrap();

        assert_eq!(
            header.convention_for(NamingTarget::Functions),
            Some(NamingConvention::Uppercase)
        );
        assert_eq!(
            header.convention_for(NamingTarget::Variables),
            Some(NamingConvention::CamelCase)
        );
    }

    #[test]
    fn rejects_unknown_duplicate_and_malformed_rules() {
        assert_eq!(
            parse_module_header("!as Naming\n!style types=PascalCase")
                .unwrap_err()
                .message,
            "unknown naming target"
        );
        assert_eq!(
            parse_module_header(
                "!as Naming\n!style variables=camelCase\n!style variables=PascalCase"
            )
            .unwrap_err()
            .message,
            "duplicate naming target"
        );
        assert_eq!(
            parse_module_header("!as Naming\n!style variables=snake_case")
                .unwrap_err()
                .message,
            "unknown naming convention"
        );
    }

    #[test]
    fn emits_non_fatal_warnings_for_nonconforming_names() {
        let header = parse_module_header(
            "!as Naming\n!style functions=camelCase, variables=UPPER_SNAKE_CASE, parameters=PascalCase",
        )
        .unwrap();

        assert_eq!(
            header.naming_warning(NamingTarget::Functions, "GetValue"),
            Some("naming style warning: functions name 'GetValue' should use camelCase".to_owned())
        );
        assert_eq!(
            header.naming_warning(NamingTarget::Variables, "MAX_COUNT"),
            None
        );
        assert_eq!(
            header.naming_warning(NamingTarget::Parameters, "userId"),
            Some("naming style warning: parameters name 'userId' should use PascalCase".to_owned())
        );
    }

    #[test]
    fn validates_each_supported_convention() {
        assert!(NamingConvention::Uppercase.matches("MAX42"));
        assert!(!NamingConvention::Uppercase.matches("MAX_COUNT"));
        assert!(NamingConvention::CamelCase.matches("userId2"));
        assert!(!NamingConvention::CamelCase.matches("UserId"));
        assert!(NamingConvention::PascalCase.matches("UserId2"));
        assert!(!NamingConvention::PascalCase.matches("userId"));
        assert!(NamingConvention::UpperSnakeCase.matches("MAX_COUNT_2"));
        assert!(!NamingConvention::UpperSnakeCase.matches("MAX__COUNT"));
    }
}
