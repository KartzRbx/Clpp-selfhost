//! Parser for the stage-0 interpreter.
//!
//! Follows the same rules as `src/parser/parser.clp`: semicolons are optional
//! (a line break ends a statement), calls/indexing only continue an expression
//! on the same line, and `Type name` on one line starts a declaration.

use crate::ast::*;
use crate::lex::{Tk, Token};
use std::rc::Rc;

pub struct Parser<'a> {
    toks: Vec<Token>,
    pos: usize,
    file: &'a str,
}

type PResult<T> = Result<T, String>;

const STATEMENT_WORDS: &[&str] = &[
    "if", "else", "while", "do", "for", "guard", "try", "catch", "break", "continue", "return",
    "link", "export", "def", "enum", "const", "auto", "match", "func",
];
const OPERATOR_WORDS: &[&str] = &["and", "or", "not", "in", "shl", "shr", "as", "true", "false", "null"];

pub fn parse(tokens: Vec<Token>, file: &str, default_name: &str) -> PResult<Program> {
    let mut parser = Parser { toks: tokens, pos: 0, file };
    parser.program(default_name)
}

impl<'a> Parser<'a> {
    // ---------------------------------------------------------------- helpers

    fn tok(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn at(&self, i: usize) -> &Token {
        &self.toks[i.min(self.toks.len() - 1)]
    }

    fn at_end(&self) -> bool {
        self.tok().kind == Tk::Eof
    }

    fn is_sym_at(&self, i: usize, s: &str) -> bool {
        let t = self.at(i);
        t.kind == Tk::Sym && t.text == s
    }

    fn is_word_at(&self, i: usize, s: &str) -> bool {
        let t = self.at(i);
        t.kind == Tk::Ident && t.text == s
    }

    fn is_sym(&self, s: &str) -> bool {
        self.is_sym_at(self.pos, s)
    }

    fn is_word(&self, s: &str) -> bool {
        self.is_word_at(self.pos, s)
    }

    fn eat_sym(&mut self, s: &str) -> bool {
        if self.is_sym(s) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_word(&mut self, s: &str) -> bool {
        if self.is_word(s) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn error<T>(&self, message: &str) -> PResult<T> {
        let t = self.tok();
        let found = if t.kind == Tk::Eof { "end of file".to_string() } else { format!("'{}'", t.text) };
        Err(format!("{}:{}:{}: {} (found {})", self.file, t.line, t.column, message, found))
    }

    fn expect_sym(&mut self, s: &str) -> PResult<()> {
        if self.eat_sym(s) {
            Ok(())
        } else {
            self.error(&format!("expected '{}'", s))
        }
    }

    fn expect_word(&mut self, s: &str) -> PResult<()> {
        if self.eat_word(s) {
            Ok(())
        } else {
            self.error(&format!("expected '{}'", s))
        }
    }

    fn expect_ident(&mut self, what: &str) -> PResult<String> {
        if self.tok().kind == Tk::Ident {
            let text = self.tok().text.clone();
            self.pos += 1;
            Ok(text)
        } else {
            self.error(&format!("expected {}", what))
        }
    }

    fn here(&self) -> Pos {
        Pos { line: self.tok().line, col: self.tok().column }
    }

    fn ident_at(&mut self, what: &str) -> PResult<(String, Pos)> {
        let pos = self.here();
        Ok((self.expect_ident(what)?, pos))
    }

    fn same_line(&self) -> bool {
        self.pos > 0 && self.tok().line == self.toks[self.pos - 1].line
    }

    fn terminator(&mut self, what: &str) -> PResult<()> {
        if self.eat_sym(";") {
            return Ok(());
        }
        if self.at_end() || self.is_sym("}") || !self.same_line() {
            return Ok(());
        }
        self.error(&format!("expected ';' or line break after {}", what))
    }

    // ---------------------------------------------------------------- module

    fn program(&mut self, default_name: &str) -> PResult<Program> {
        let mut module_name = default_name.to_string();
        if self.is_sym("!") && self.is_word_at(self.pos + 1, "as") {
            self.pos += 2;
            module_name = self.expect_ident("module name after '!as'")?;
            self.eat_sym(";");
        }
        // `!style` directives only matter to the self-hosted checker.
        while self.is_sym("!") && self.is_word_at(self.pos + 1, "style") {
            let line = self.tok().line;
            while !self.at_end() && self.tok().line == line {
                self.pos += 1;
            }
        }

        let mut items = Vec::new();
        while !self.at_end() {
            if self.eat_sym(";") {
                continue;
            }
            items.push(self.item()?);
        }
        Ok(Program { module_name, items })
    }

    fn item(&mut self) -> PResult<Item> {
        let line = self.tok().line;
        if self.eat_word("link") {
            if self.eat_sym("@") {
                let mut name = format!("@{}", self.expect_ident("standard module name after '@'")?);
                while self.eat_sym(".") {
                    name.push('.');
                    name.push_str(&self.expect_ident("module name after '.'")?);
                }
                let mut pos = Pos { line, col: 1 };
                let alias = if self.eat_word("as") {
                    let (a, p) = self.ident_at("link alias")?;
                    pos = p;
                    Some(a)
                } else {
                    None
                };
                self.terminator("link")?;
                return Ok(Item::Link { path: name, alias, line, pos });
            }
            if self.tok().kind != Tk::Str {
                return self.error("expected quoted path after 'link'");
            }
            let path = self.tok().text.clone();
            self.pos += 1;
            let mut pos = Pos { line, col: 1 };
            let alias = if self.eat_word("as") {
                let (a, p) = self.ident_at("link alias")?;
                pos = p;
                Some(a)
            } else {
                None
            };
            self.terminator("link")?;
            return Ok(Item::Link { path, alias, line, pos });
        }

        let exported = self.eat_word("export");
        // `template<typename T> T First(array<T> values)`: type parameters only
        // matter to the type checker.
        if self.eat_word("template") {
            self.type_params()?;
        }

        if self.eat_word("def") {
            return self.def_item(exported);
        }
        if self.eat_word("enum") {
            self.expect_word("class")?;
            let (name, pos) = self.ident_at("enum name")?;
            self.expect_sym("{")?;
            let mut variants = Vec::new();
            let mut variant_pos = Vec::new();
            while !self.is_sym("}") {
                let (v, p) = self.ident_at("enum member")?;
                variants.push(v);
                variant_pos.push(p);
                if !self.eat_sym(",") {
                    break;
                }
            }
            self.expect_sym("}")?;
            self.eat_sym(";");
            return Ok(Item::Enum { decl: Rc::new(EnumDecl { name, variants, pos, variant_pos }), exported });
        }

        let is_const = self.is_word("const");
        if is_const || self.is_word("auto") || self.looks_like_decl(self.pos) {
            if !is_const && !self.is_word("auto") {
                // Could be a function: Type Name(
                let save = self.pos;
                let ret = self.type_name()?;
                let (name, pos) = self.ident_at("declaration name")?;
                if self.is_sym("(") {
                    let decl = self.function_rest(name, ret, line, pos)?;
                    return Ok(Item::Function { decl, exported });
                }
                self.pos = save;
            }
            let stmt = self.var_decl()?;
            return Ok(Item::Global { stmt, exported });
        }

        if exported {
            return self.error("expected a declaration after 'export'");
        }
        Ok(Item::Stmt(self.statement()?))
    }

    /// `<T, U: Bound>` or `<typename T>`.
    fn type_params(&mut self) -> PResult<Vec<String>> {
        self.expect_sym("<")?;
        let mut names = Vec::new();
        loop {
            self.eat_word("typename");
            names.push(self.expect_ident("type parameter name")?);
            if self.eat_sym(":") {
                self.type_name()?;
            }
            if !self.eat_sym(",") {
                break;
            }
        }
        self.expect_sym(">")?;
        Ok(names)
    }

    fn def_item(&mut self, exported: bool) -> PResult<Item> {
        let (name, pos) = self.ident_at("type name after 'def'")?;
        let type_params = if self.is_sym("<") { self.type_params()? } else { Vec::new() };
        if self.eat_sym("=") {
            self.skip_type_expr()?;
            self.terminator("type declaration")?;
            return Ok(Item::TypeAlias { name, exported, pos });
        }
        if self.is_sym(":") {
            return self.error("def inheritance is not supported by clpp-boot yet");
        }
        self.expect_sym("{")?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while !self.is_sym("}") {
            if self.at_end() {
                return self.error("expected '}' to close def body");
            }
            if self.eat_sym(";") {
                continue;
            }
            self.eat_word("public");
            self.eat_word("private");
            let line = self.tok().line;
            let ty = self.type_name()?;
            let (member, member_pos) = self.ident_at("field or method name")?;
            if self.is_sym("(") {
                methods.push(self.function_rest(member, ty, line, member_pos)?);
            } else {
                let default = if self.eat_sym("=") { Some(self.expr(0)?) } else { None };
                self.terminator("field")?;
                fields.push(FieldDecl { name: member, ty, default, pos: member_pos });
            }
        }
        let end_line = self.tok().line;
        self.expect_sym("}")?;
        self.eat_sym(";");
        Ok(Item::Def { decl: Rc::new(DefDecl { name, type_params, pos, end_line, fields, methods }), exported })
    }

    fn function_rest(&mut self, name: String, ret: TypeName, line: usize, pos: Pos) -> PResult<Rc<FnDecl>> {
        self.expect_sym("(")?;
        let mut params = Vec::new();
        let mut param_types = Vec::new();
        if !self.is_sym(")") {
            loop {
                param_types.push(self.type_name()?);
                params.push(self.expect_ident("parameter name")?);
                if !self.eat_sym(",") {
                    break;
                }
            }
        }
        self.expect_sym(")")?;
        let body = self.block()?;
        let end_line = self.toks[self.pos - 1].line;
        Ok(Rc::new(FnDecl { name, params, param_types, ret, body, line, pos, end_line }))
    }

    // ---------------------------------------------------------------- types

    fn is_type_token(&self, i: usize) -> bool {
        let t = self.at(i);
        t.kind == Tk::Ident || (t.kind == Tk::Sym && matches!(t.text.as_str(), "." | "," | "<" | ">"))
    }

    fn looks_like_decl(&self, i: usize) -> bool {
        let t = self.at(i);
        if t.kind != Tk::Ident
            || STATEMENT_WORDS.contains(&t.text.as_str())
            || OPERATOR_WORDS.contains(&t.text.as_str())
        {
            return false;
        }
        let mut j = i + 1;
        while self.is_sym_at(j, ".") && self.at(j + 1).kind == Tk::Ident {
            j += 2;
        }
        if self.is_sym_at(j, "<") {
            let mut depth = 0i32;
            loop {
                if !self.is_type_token(j) {
                    return false;
                }
                if self.is_sym_at(j, "<") {
                    depth += 1;
                } else if self.is_sym_at(j, ">") {
                    depth -= 1;
                    if depth == 0 {
                        j += 1;
                        break;
                    }
                }
                j += 1;
            }
        }
        let name = self.at(j);
        name.kind == Tk::Ident
            && !OPERATOR_WORDS.contains(&name.text.as_str())
            && name.line == self.at(j - 1).line
    }

    fn type_name(&mut self) -> PResult<TypeName> {
        let mut base = self.expect_ident("type name")?;
        while self.is_sym(".") && self.at(self.pos + 1).kind == Tk::Ident {
            base.push('.');
            base.push_str(&self.at(self.pos + 1).text.clone());
            self.pos += 2;
        }
        let mut args = Vec::new();
        if self.eat_sym("<") {
            loop {
                args.push(self.type_name()?);
                if !self.eat_sym(",") {
                    break;
                }
            }
            self.expect_sym(">")?;
        }
        Ok(TypeName { base, args })
    }

    /// Structural type expressions only matter to the self-hosted checker.
    fn skip_type_expr(&mut self) -> PResult<()> {
        loop {
            if self.eat_word("typeof") {
                self.expect_sym("(")?;
                self.expr(0)?;
                self.expect_sym(")")?;
            } else if self.eat_sym("{") {
                while !self.is_sym("}") {
                    if self.at_end() {
                        return self.error("expected '}' after object type");
                    }
                    self.expect_ident("field name in object type")?;
                    self.expect_sym(":")?;
                    self.skip_type_expr()?;
                    if !self.eat_sym(",") {
                        self.eat_sym(";");
                    }
                }
                self.expect_sym("}")?;
            } else if self.tok().kind == Tk::Str {
                self.pos += 1;
            } else {
                self.type_name()?;
            }
            if !(self.eat_sym("&") || self.eat_sym("|")) {
                return Ok(());
            }
        }
    }

    // ---------------------------------------------------------------- statements

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect_sym("{")?;
        let mut stmts = Vec::new();
        while !self.is_sym("}") {
            if self.at_end() {
                return self.error("expected '}' to close block");
            }
            if self.eat_sym(";") {
                continue;
            }
            stmts.push(self.statement()?);
        }
        self.expect_sym("}")?;
        Ok(stmts)
    }

    fn condition(&mut self, what: &str) -> PResult<Expr> {
        if !self.eat_sym("(") {
            return self.error(&format!("expected '(' after {}", what));
        }
        let cond = self.expr(0)?;
        self.expect_sym(")")?;
        Ok(cond)
    }

    fn var_decl(&mut self) -> PResult<Stmt> {
        let line = self.tok().line;
        let is_const = self.eat_word("const");
        let ty = if self.eat_word("auto") { None } else { Some(self.type_name()?) };
        let (name, pos) = self.ident_at("variable name")?;
        if self.is_sym("(") {
            return self.error("functions can only be declared at the top level");
        }
        let init = if self.eat_sym("=") { Some(self.expr(0)?) } else { None };
        if is_const && init.is_none() {
            return self.error(&format!("const declaration '{}' requires an initializer", name));
        }
        self.terminator("variable declaration")?;
        Ok(Stmt { kind: StmtKind::Var { name, ty, init, pos }, line })
    }

    fn statement(&mut self) -> PResult<Stmt> {
        let line = self.tok().line;
        let kind = if self.is_sym("{") {
            StmtKind::Block(self.block()?)
        } else if self.eat_word("if") {
            return self.if_rest(line);
        } else if self.eat_word("while") {
            let cond = self.condition("while")?;
            StmtKind::While(cond, self.block()?)
        } else if self.eat_word("do") {
            let body = self.block()?;
            self.expect_word("while")?;
            let cond = self.condition("do-while")?;
            self.terminator("do-while")?;
            StmtKind::DoWhile(body, cond)
        } else if self.eat_word("for") {
            self.expect_sym("(")?;
            let ty = if self.eat_word("auto") { None } else { Some(self.type_name()?) };
            let (name, pos) = self.ident_at("loop variable name")?;
            if !self.eat_word("in") {
                return self.error("expected 'in' after loop variable; CL++ loops use 'for (Type item in values)'");
            }
            let iterable = self.expr(0)?;
            self.expect_sym(")")?;
            StmtKind::ForIn { name, ty, iterable, body: self.block()?, pos }
        } else if self.eat_word("match") {
            let subject = self.condition("match")?;
            self.expect_sym("{")?;
            let mut arms = Vec::new();
            while !self.is_sym("}") {
                if self.at_end() {
                    return self.error("expected '}' to close match");
                }
                if self.eat_sym(",") || self.eat_sym(";") {
                    continue;
                }
                let arm_line = self.tok().line;
                let mut patterns = Vec::new();
                loop {
                    if self.is_word("_") {
                        self.pos += 1;
                        patterns.push(None);
                    } else {
                        patterns.push(Some(self.expr(3)?));
                    }
                    if !self.eat_sym(",") {
                        break;
                    }
                }
                self.expect_sym("=>")?;
                let body = if self.is_sym("{") {
                    self.block()?
                } else {
                    let stmt_line = self.tok().line;
                    let e = self.expr(0)?;
                    vec![Stmt { kind: StmtKind::Expr(e), line: stmt_line }]
                };
                arms.push(MatchArm { patterns, body, line: arm_line });
            }
            self.expect_sym("}")?;
            StmtKind::Match(subject, arms)
        } else if self.eat_word("guard") {
            let cond = self.condition("guard")?;
            self.expect_word("else")?;
            StmtKind::Guard(cond, self.block()?)
        } else if self.eat_word("try") {
            let body = self.block()?;
            self.expect_word("catch")?;
            let binding = if self.eat_sym("(") {
                let name = self.expect_ident("error name in catch")?;
                self.expect_sym(")")?;
                Some(name)
            } else {
                None
            };
            StmtKind::Try { body, binding, handler: self.block()? }
        } else if self.eat_word("break") {
            self.terminator("break")?;
            StmtKind::Break
        } else if self.eat_word("continue") {
            self.terminator("continue")?;
            StmtKind::Continue
        } else if self.eat_word("return") {
            let value = if self.is_sym(";") || self.is_sym("}") || self.at_end() || !self.same_line() {
                None
            } else {
                Some(self.expr(0)?)
            };
            self.terminator("return")?;
            StmtKind::Return(value)
        } else if self.is_word("else") || self.is_word("catch") {
            return self.error("without a matching statement");
        } else if self.is_word("const") || self.is_word("auto") || self.looks_like_decl(self.pos) {
            return self.var_decl();
        } else {
            let e = self.expr(0)?;
            self.terminator("expression")?;
            StmtKind::Expr(e)
        };
        Ok(Stmt { kind, line })
    }

    fn if_rest(&mut self, line: usize) -> PResult<Stmt> {
        let cond = self.condition("if")?;
        let then = self.block()?;
        let otherwise = if self.eat_word("else") {
            if self.is_word("if") {
                let else_line = self.tok().line;
                self.pos += 1;
                Some(vec![self.if_rest(else_line)?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt { kind: StmtKind::If(cond, then, otherwise), line })
    }

    // ---------------------------------------------------------------- expressions

    fn precedence(&self) -> u8 {
        let t = self.tok();
        if t.kind != Tk::Sym && t.kind != Tk::Ident {
            return 0;
        }
        match t.text.as_str() {
            "=" | "+=" | "-=" | "*=" | "/=" | "%=" if t.kind == Tk::Sym => 1,
            "or" if t.kind == Tk::Ident => 3,
            "and" if t.kind == Tk::Ident => 4,
            "==" | "!=" | "<" | "<=" | ">" | ">=" if t.kind == Tk::Sym => 5,
            ".." if t.kind == Tk::Sym => 6,
            "&" | "|" | "^" if t.kind == Tk::Sym => 7,
            "shl" | "shr" if t.kind == Tk::Ident => 7,
            "+" | "-" | ".:" if t.kind == Tk::Sym => 8,
            "*" | "/" | "%" if t.kind == Tk::Sym => 9,
            _ => 0,
        }
    }

    pub fn expr(&mut self, min: u8) -> PResult<Expr> {
        let mut left = self.prefix()?;
        loop {
            if self.at_end() {
                break;
            }
            let line = self.tok().line;
            if self.is_sym("(") && self.same_line() {
                self.pos += 1;
                let mut args = Vec::new();
                if !self.is_sym(")") {
                    loop {
                        args.push(self.expr(0)?);
                        if !self.eat_sym(",") {
                            break;
                        }
                    }
                }
                self.expect_sym(")")?;
                left = Expr { kind: ExprKind::Call(Box::new(left), args), line };
                continue;
            }
            if self.eat_sym(".") {
                let name = self.expect_ident("member name after '.'")?;
                left = Expr { kind: ExprKind::Member(Box::new(left), name), line };
                continue;
            }
            if self.eat_sym("::") {
                let name = self.expect_ident("name after '::'")?;
                left = Expr { kind: ExprKind::Scope(Box::new(left), name), line };
                continue;
            }
            if self.is_sym("{") && self.same_line() && self.looks_like_new(&left) {
                left = self.new_rest(left)?;
                continue;
            }
            if self.is_sym("[") && self.same_line() {
                self.pos += 1;
                let index = self.expr(0)?;
                self.expect_sym("]")?;
                left = Expr { kind: ExprKind::Index(Box::new(left), Box::new(index)), line };
                continue;
            }
            if (self.is_sym("++") || self.is_sym("--")) && self.same_line() && 11 >= min {
                let op = self.tok().text.clone();
                self.pos += 1;
                left = Expr { kind: ExprKind::Postfix(op, Box::new(left)), line };
                continue;
            }
            if self.is_sym("?") && 2 >= min {
                self.pos += 1;
                let yes = self.expr(0)?;
                self.expect_sym(":")?;
                let no = self.expr(2)?;
                left = Expr { kind: ExprKind::Ternary(Box::new(left), Box::new(yes), Box::new(no)), line };
                continue;
            }
            let prec = self.precedence();
            if prec == 0 || prec < min {
                break;
            }
            let op = self.tok().text.clone();
            self.pos += 1;
            if prec == 1 {
                let value = self.expr(1)?;
                left = Expr { kind: ExprKind::Assign(op, Box::new(left), Box::new(value)), line };
            } else {
                let right = self.expr(prec + 1)?;
                left = Expr { kind: ExprKind::Binary(op, Box::new(left), Box::new(right)), line };
            }
        }
        Ok(left)
    }

    /// `` `a ${b} c` `` becomes `"a " .: b .: " c"`.
    fn template(&mut self, t: &Token) -> PResult<Expr> {
        let line = t.line;
        let chars: Vec<char> = t.text.chars().collect();
        let mut result = Expr { kind: ExprKind::Str(Rc::from("")), line };
        let mut text = String::new();
        let join = |left: Expr, right: Expr| Expr { kind: ExprKind::Binary(".:".into(), Box::new(left), Box::new(right)), line };
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' && i + 1 < chars.len() {
                text.push(match chars[i + 1] {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    other => other,
                });
                i += 2;
                continue;
            }
            if c == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                let mut depth = 1;
                let mut j = i + 2;
                while j < chars.len() && depth > 0 {
                    match chars[j] {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                let inner: String = chars[i + 2..j - 1].iter().collect();
                if inner.trim().is_empty() {
                    return Err(format!("{}:{}:{}: empty ${{}} in template string", self.file, t.line, t.column));
                }
                let tokens = crate::lex::lex(&inner, self.file).map_err(|m| format!("{} (inside a template string on line {})", m, line))?;
                let mut sub = Parser { toks: tokens, pos: 0, file: self.file };
                let value = sub.expr(0)?;
                if !sub.at_end() {
                    return Err(format!("{}:{}:{}: unexpected text inside ${{}} of a template string", self.file, t.line, t.column));
                }
                if !text.is_empty() {
                    result = join(result, Expr { kind: ExprKind::Str(Rc::from(text.as_str())), line });
                    text.clear();
                }
                result = join(result, value);
                i = j;
                continue;
            }
            text.push(c);
            i += 1;
        }
        if !text.is_empty() || matches!(&result.kind, ExprKind::Str(_)) {
            result = join(result, Expr { kind: ExprKind::Str(Rc::from(text.as_str())), line });
        }
        Ok(result)
    }

    fn looks_like_new(&self, left: &Expr) -> bool {
        if !matches!(left.kind, ExprKind::Ident(_) | ExprKind::Member(_, _)) {
            return false;
        }
        self.is_sym_at(self.pos + 1, "}")
            || (self.at(self.pos + 1).kind == Tk::Ident && self.is_sym_at(self.pos + 2, "="))
    }

    fn new_rest(&mut self, type_path: Expr) -> PResult<Expr> {
        let line = self.tok().line;
        self.expect_sym("{")?;
        let mut fields: Vec<(String, Expr)> = Vec::new();
        while !self.is_sym("}") {
            if self.at_end() {
                return self.error("expected '}' after object construction");
            }
            let name = self.expect_ident("field name")?;
            self.expect_sym("=")?;
            let value = self.expr(2)?;
            if fields.iter().any(|(n, _)| *n == name) {
                return self.error(&format!("field '{}' initialized more than once", name));
            }
            fields.push((name, value));
            if self.eat_sym(",") || self.is_sym("}") || !self.same_line() {
                continue;
            }
            return self.error("expected ',' or line break after field initializer");
        }
        self.expect_sym("}")?;
        Ok(Expr { kind: ExprKind::New(Box::new(type_path), fields), line })
    }

    /// `(Type value)`: a type name followed directly by the start of a value.
    /// `(a b)` is never a valid expression, so this cannot hide one.
    fn looks_like_typed_expr(&self) -> bool {
        let start = self.pos + 1;
        let first = self.at(start);
        if first.kind != Tk::Ident
            || STATEMENT_WORDS.contains(&first.text.as_str())
            || OPERATOR_WORDS.contains(&first.text.as_str())
        {
            return false;
        }
        let mut j = start + 1;
        while self.is_sym_at(j, ".") && self.at(j + 1).kind == Tk::Ident {
            j += 2;
        }
        if self.is_sym_at(j, "<") {
            let mut depth = 0i32;
            loop {
                if !self.is_type_token(j) {
                    return false;
                }
                if self.is_sym_at(j, "<") {
                    depth += 1;
                } else if self.is_sym_at(j, ">") {
                    depth -= 1;
                    if depth == 0 {
                        j += 1;
                        break;
                    }
                }
                j += 1;
            }
        }
        let next = self.at(j);
        match next.kind {
            Tk::Int | Tk::Float | Tk::Str => true,
            Tk::Ident => !matches!(next.text.as_str(), "and" | "or" | "in" | "shl" | "shr" | "as"),
            Tk::Sym => match next.text.as_str() {
                // `(len(x) > 1)` is a call: the `(` touches the name.
                "(" => {
                    let previous = self.at(j - 1);
                    next.line != previous.line || next.column > previous.column + previous.text.chars().count()
                }
                "@" => true,
                "-" => crate::stdlib::is_numeric_type(&first.text) && matches!(self.at(j + 1).kind, Tk::Int | Tk::Float),
                _ => false,
            },
            Tk::Eof | Tk::Template => matches!(next.kind, Tk::Template),
        }
    }

    fn looks_like_generic_new(&self) -> bool {
        if self.tok().kind != Tk::Ident || !self.is_sym_at(self.pos + 1, "<") {
            return false;
        }
        let mut j = self.pos + 1;
        let mut depth = 0i32;
        loop {
            if !self.is_type_token(j) {
                return false;
            }
            if self.is_sym_at(j, "<") {
                depth += 1;
            } else if self.is_sym_at(j, ">") {
                depth -= 1;
                if depth == 0 {
                    return self.is_sym_at(j + 1, "{") && self.at(j + 1).line == self.at(j).line;
                }
            }
            j += 1;
        }
    }

    fn looks_like_typed_collection(&self) -> bool {
        if self.tok().kind != Tk::Ident || !self.is_sym_at(self.pos + 1, "<") {
            return false;
        }
        let mut j = self.pos + 1;
        let mut depth = 0i32;
        loop {
            if !self.is_type_token(j) {
                return false;
            }
            if self.is_sym_at(j, "<") {
                depth += 1;
            } else if self.is_sym_at(j, ">") {
                depth -= 1;
                if depth == 0 {
                    return self.is_sym_at(j + 1, "(");
                }
            }
            j += 1;
        }
    }

    fn prefix(&mut self) -> PResult<Expr> {
        let t = self.tok().clone();
        let line = t.line;
        let mk = |kind| Ok(Expr { kind, line });

        if (t.kind == Tk::Sym && matches!(t.text.as_str(), "!" | "-" | "+" | "~"))
            || (t.kind == Tk::Ident && t.text == "not")
        {
            self.pos += 1;
            let operand = self.expr(10)?;
            let op = if t.text == "!" { "not".to_string() } else { t.text.clone() };
            return mk(ExprKind::Unary(op, Box::new(operand)));
        }
        match t.kind {
            Tk::Int => {
                self.pos += 1;
                let clean: String = t.text.chars().filter(|c| *c != '_').collect();
                let parsed = if let Some(hex) = clean.strip_prefix("0x").or(clean.strip_prefix("0X")) {
                    u64::from_str_radix(hex, 16).map(|v| v as i64)
                } else if let Some(bin) = clean.strip_prefix("0b").or(clean.strip_prefix("0B")) {
                    u64::from_str_radix(bin, 2).map(|v| v as i64)
                } else {
                    clean.parse::<i64>()
                };
                return match parsed {
                    Ok(v) => mk(ExprKind::Int(v)),
                    Err(_) => Err(format!("{}:{}:{}: invalid integer literal '{}'", self.file, t.line, t.column, t.text)),
                };
            }
            Tk::Float => {
                self.pos += 1;
                let clean: String = t.text.chars().filter(|c| *c != '_').collect();
                return mk(ExprKind::Float(clean.parse::<f64>().unwrap_or(f64::NAN)));
            }
            Tk::Str => {
                self.pos += 1;
                return mk(ExprKind::Str(Rc::from(t.text.as_str())));
            }
            Tk::Template => {
                self.pos += 1;
                return self.template(&t);
            }
            _ => {}
        }
        if self.eat_sym("@") {
            if self.eat_word("this") {
                return mk(ExprKind::This);
            }
            // `@Name` is shorthand for `@this.Name`.
            if self.tok().kind == Tk::Ident && self.same_line() {
                let name = self.tok().text.clone();
                self.pos += 1;
                return mk(ExprKind::Member(Box::new(Expr { kind: ExprKind::This, line }), name));
            }

            return self.error("expected 'this' or a member name after '@'");
        }
        if self.is_sym("(") && self.looks_like_typed_expr() {
            self.pos += 1;
            let ty = self.type_name()?;
            let value = self.expr(0)?;
            self.expect_sym(")")?;

            return mk(ExprKind::Typed(ty, Box::new(value)));
        }
        if self.eat_sym("(") {
            let e = self.expr(0)?;
            self.expect_sym(")")?;

            return Ok(e);
        }
        if self.is_sym("{") {
            return self.collection_literal();
        }
        if t.kind == Tk::Ident {
            match t.text.as_str() {
                "true" => {
                    self.pos += 1;
                    return mk(ExprKind::Bool(true));
                }
                "false" => {
                    self.pos += 1;
                    return mk(ExprKind::Bool(false));
                }
                "null" => {
                    self.pos += 1;
                    return mk(ExprKind::Null);
                }
                "func" => {
                    let pos = self.here();
                    self.pos += 1;
                    self.expect_sym("(")?;
                    let mut params = Vec::new();
                    let mut param_types = Vec::new();
                    if !self.is_sym(")") {
                        loop {
                            param_types.push(self.type_name()?);
                            params.push(self.expect_ident("parameter name")?);
                            if !self.eat_sym(",") {
                                break;
                            }
                        }
                    }
                    self.expect_sym(")")?;
                    let ret = if self.eat_sym("->") { self.type_name()? } else { TypeName { base: "void".into(), args: Vec::new() } };
                    let body = self.block()?;
                    let end_line = self.toks[self.pos - 1].line;
                    let decl = FnDecl { name: "<lambda>".into(), params, param_types, ret, body, line, pos, end_line };
                    return mk(ExprKind::Lambda(Rc::new(decl)));
                }
                _ => {}
            }
            if self.looks_like_generic_new() {
                // `Box<number> { Value = 1 }`: the type arguments are for the checker.
                self.type_name()?;
                return mk(ExprKind::Ident(t.text.clone()));
            }
            if self.looks_like_typed_collection() {
                let ty = self.type_name()?;
                self.expect_sym("(")?;
                let mut args = Vec::new();
                if !self.is_sym(")") {
                    loop {
                        args.push(self.expr(0)?);
                        if !self.eat_sym(",") {
                            break;
                        }
                    }
                }
                self.expect_sym(")")?;
                return mk(ExprKind::TypedCollection(ty, args));
            }
            self.pos += 1;
            return mk(ExprKind::Ident(t.text.clone()));
        }
        self.error("expected expression")
    }

    fn collection_literal(&mut self) -> PResult<Expr> {
        let line = self.tok().line;
        self.expect_sym("{")?;
        let first = self.tok().clone();
        let is_dict = if (first.kind == Tk::Ident || first.kind == Tk::Str) && self.is_sym_at(self.pos + 1, "=") {
            true
        } else if self.is_sym("[") {
            let mut j = self.pos;
            let mut depth = 0;
            loop {
                if self.at(j).kind == Tk::Eof {
                    break false;
                }

                if self.is_sym_at(j, "[") {
                    depth += 1;
                } else if self.is_sym_at(j, "]") {
                    depth -= 1;
                    if depth == 0 {
                        break self.is_sym_at(j + 1, "=");
                    }
                }
                
                j += 1;
            }
        } else {
            false
        };

        if is_dict {
            let mut entries = Vec::new();
            while !self.is_sym("}") {
                let key = if self.eat_sym("[") {
                    let k = self.expr(0)?;
                    self.expect_sym("]")?;
                    DictKey::Expr(k)
                } else if self.tok().kind == Tk::Ident {
                    let name = self.tok().text.clone();
                    self.pos += 1;
                    DictKey::Name(name)
                } else if self.tok().kind == Tk::Str {
                    let text = self.tok().text.clone();
                    let key_line = self.tok().line;
                    self.pos += 1;
                    DictKey::Expr(Expr { kind: ExprKind::Str(Rc::from(text.as_str())), line: key_line })
                } else {
                    return self.error("expected dictionary key");
                };
                self.expect_sym("=")?;
                let value = self.expr(2)?;
                entries.push((key, value));
                if self.eat_sym(",") || self.is_sym("}") || !self.same_line() {
                    continue;
                }
                return self.error("expected ',' after dictionary entry");
            }
            self.expect_sym("}")?;
            return Ok(Expr { kind: ExprKind::DictLit(entries), line });
        }

        let mut items = Vec::new();
        while !self.is_sym("}") {
            items.push(self.expr(0)?);
            if !self.eat_sym(",") {
                break;
            }
        }
        self.expect_sym("}")?;
        Ok(Expr { kind: ExprKind::ArrayLit(items), line })
    }
}
