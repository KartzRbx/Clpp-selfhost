//! Static analysis shared by `clpp-boot check` and the language server:
//! diagnostics, a symbol index per file, and name resolution at a position.

use crate::ast::*;
use crate::lex::{self, Tk, Token};
use crate::parse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error = 1,
    Warning = 2,
    Info = 3,
}

#[derive(Debug, Clone)]
pub struct Diag {
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub severity: Severity,
    pub message: String,
    /// What to change, shown under the error.
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymKind {
    Function,
    Def,
    Field,
    Method,
    Enum,
    Variant,
    Variable,
    TypeAlias,
    Module,
    Param,
    Local,
    Builtin,
}

#[derive(Debug, Clone)]
pub struct Sym {
    pub name: String,
    pub kind: SymKind,
    pub pos: Pos,
    pub end_line: usize,
    pub signature: String,
    pub docs: Vec<(String, String)>,
    pub exported: bool,
    /// Parameter count for functions and methods.
    pub arity: Option<usize>,
    /// Declared type for variables, fields and parameters.
    pub ty: Option<String>,
    /// Fields and methods of a def, variants of an enum.
    pub children: Vec<Sym>,
    /// Parameter types of functions and methods, as written.
    pub params: Vec<String>,
    /// File that declares the symbol (for links: the linked file).
    pub path: PathBuf,
}

#[derive(Clone)]
pub struct Link {
    pub alias: String,
    pub target: PathBuf,
    pub native: bool,
    pub line: usize,
    pub pos: Pos,
}

#[derive(Clone)]
pub struct FileIndex {
    pub path: PathBuf,
    pub symbols: Vec<Sym>,
    pub links: Vec<Link>,
}

#[derive(Clone)]
pub struct Analysis {
    pub path: PathBuf,
    pub tokens: Vec<Token>,
    pub program: Option<Program>,
    pub index: Option<FileIndex>,
    pub diags: Vec<Diag>,
    /// Indexes of linked files, keyed by alias.
    pub linked: HashMap<String, FileIndex>,
}

pub const BUILTINS: &[(&str, &str, &str)] = &[
    ("post", "void post(any value)", "Prints a value and a line break to standard output."),
    ("warn", "void warn(any value)", "Prints a warning to standard error."),
    ("report", "void report(string message)", "Raises an error. `try`/`catch` can catch it."),
    ("len", "number len(string | array | dictionary value)", "Number of characters, elements or entries."),
    ("push", "void push(array<T> items, T value)", "Appends a value to the end of an array."),
    ("pop", "T pop(array<T> items)", "Removes and returns the last element."),
    ("insert", "void insert(array<T> items, number index, T value)", "Inserts a value at an index."),
    ("remove", "T remove(array<T> items, number index)", "Removes and returns the element at an index (or a dictionary key)."),
    ("find", "number find(array<T> items, T value)", "Index of a value, or -1. With a string, the index of a substring."),
    ("sort", "void sort(array<T> items)", "Sorts numbers or strings in place."),
    ("args", "array<string> args()", "Command-line arguments passed after the file name."),
];

pub const KEYWORDS: &[&str] = &[
    "if", "else", "while", "do", "for", "in", "guard", "try", "catch", "break", "continue", "return",
    "link", "as", "export", "def", "enum", "class", "const", "auto", "true", "false", "null", "and",
    "or", "not", "shl", "shr", "typeof", "public", "private", "number", "float", "string", "bool",
    "void", "array", "dictionary",
];

// ==================================================================== loading

pub type Loader<'a> = &'a dyn Fn(&Path) -> Option<String>;

fn error_position(message: &str, file: &str) -> (usize, usize, String) {
    // Messages look like "<file>:<line>:<col>: text".
    let rest = message.strip_prefix(file).and_then(|r| r.strip_prefix(':')).unwrap_or(message);
    let mut parts = rest.splitn(3, ':');
    let line = parts.next().and_then(|p| p.trim().parse().ok());
    let col = parts.next().and_then(|p| p.trim().parse().ok());
    match (line, col, parts.next()) {
        (Some(l), Some(c), Some(text)) => (l, c, text.trim().to_string()),
        _ => (1, 1, message.to_string()),
    }
}

const FILE_TAG: &str = "<source>";

/// Tokens that survive a lexing error: everything up to the bad character.
fn tokens_for(text: &str) -> (Vec<Token>, Option<Diag>) {
    match lex::lex(text, FILE_TAG) {
        Ok(t) => (t, None),
        Err(message) => {
            let (line, col, text_msg) = error_position(&message, FILE_TAG);
            let diag = Diag { line, col, len: 1, severity: Severity::Error, message: text_msg, help: None };
            (vec![Token { kind: Tk::Eof, text: String::new(), line, column: col, start: 0, end: 0 }], Some(diag))
        }
    }
}

fn parse_text(text: &str, stem: &str) -> (Vec<Token>, Option<Program>, Option<Diag>) {
    let (tokens, lex_error) = tokens_for(text);
    if lex_error.is_some() {
        return (tokens, None, lex_error);
    }
    match parse::parse(tokens.clone(), FILE_TAG, stem) {
        Ok(program) => (tokens, Some(program), None),
        Err(message) => {
            let (line, col, text_msg) = error_position(&message, FILE_TAG);
            let len = tokens
                .iter()
                .find(|t| t.line == line && t.column == col)
                .map(|t| t.text.chars().count().max(1))
                .unwrap_or(1);
            (tokens, None, Some(Diag { line, col, len, severity: Severity::Error, message: text_msg, help: None }))
        }
    }
}

pub fn analyze(path: &Path, text: &str, loader: Loader) -> Analysis {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Main").to_string();
    let (tokens, program, error) = parse_text(text, &stem);
    let mut analysis = Analysis {
        path: path.to_path_buf(),
        tokens,
        program: None,
        index: None,
        diags: Vec::new(),
        linked: HashMap::new(),
    };
    if let Some(d) = error {
        analysis.diags.push(d);
    }
    let program = match program {
        Some(p) => p,
        None => return analysis,
    };
    let index = build_index(path, &program, text);
    analysis.diags.extend(annotations(text));

    for link in &index.links {
        if link.native {
            if crate::stdlib::module_alias(&link.target.to_string_lossy()).is_none() {
                analysis.diags.push(Diag {
                    line: link.line,
                    col: 1,
                    len: 4,
                    severity: Severity::Error,
                    help: None,
                    message: format!("unknown standard module '{}'", link.target.display()),
                });
            }
            analysis.linked.insert(link.alias.clone(), native_index(&link.target));
            continue;
        }
        match loader(&link.target) {
            None => analysis.diags.push(Diag {
                line: link.line,
                col: 1,
                len: 4,
                severity: Severity::Error,
                help: None,
                message: format!("linked file not found: {}", link.target.display()),
            }),
            Some(linked_text) => {
                let linked_stem = link.target.file_stem().and_then(|s| s.to_str()).unwrap_or("Module").to_string();
                let (_, linked_program, linked_error) = parse_text(&linked_text, &linked_stem);
                if let Some(e) = linked_error {
                    analysis.diags.push(Diag {
                        line: link.line,
                        col: 1,
                        len: 4,
                        severity: Severity::Error,
                        help: None,
                        message: format!(
                            "{} has an error at {}:{}: {}",
                            link.target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                            e.line,
                            e.col,
                            e.message
                        ),
                    });
                }
                if let Some(p) = linked_program {
                    analysis.linked.insert(link.alias.clone(), build_index(&link.target, &p, &linked_text));
                }
            }
        }
    }

    let mut checker = Checker { a: &analysis, index: &index, diags: Vec::new(), scopes: Vec::new(), def: None };
    checker.check_program(&program);
    let diags = checker.diags;
    let mut types = crate::types::TypeChecker::new(&analysis, &index);
    types.check(&program);
    let type_diags = types.diags;
    analysis.diags.extend(diags);
    analysis.diags.extend(type_diags);
    analysis.diags.sort_by_key(|d| (d.line, d.col));
    analysis.diags.dedup_by(|a, b| a.line == b.line && a.col == b.col && a.message == b.message);
    analysis.index = Some(index);
    analysis.program = Some(program);
    analysis
}

/// A member of a def, looking through its base defs (`def Mage:Entity`).
pub fn member_in_chain<'a>(def: &'a Sym, index: &'a FileIndex, linked: &'a HashMap<String, FileIndex>, name: &str) -> Option<&'a Sym> {
    let mut current = def;
    let mut ctx = index;
    for _ in 0..16 {
        if let Some(m) = current.children.iter().find(|c| c.name == name) {
            return Some(m);
        }
        let (next, base) = base_of(current, ctx, linked)?;
        current = base;
        ctx = next;
    }
    None
}

/// The base def of `def`, with the file that declares it.
pub fn base_of<'a>(def: &Sym, ctx: &'a FileIndex, linked: &'a HashMap<String, FileIndex>) -> Option<(&'a FileIndex, &'a Sym)> {
    let base = def.ty.as_deref()?;
    let (next, name) = match base.split_once('.') {
        Some((alias, n)) => {
            let link = ctx.links.iter().find(|l| l.alias == alias)?;
            (linked.values().find(|l| l.path == link.target)?, n)
        }
        None => (ctx, base),
    };
    next.find(name).filter(|s| s.kind == SymKind::Def).map(|s| (next, s))
}

/// Every member of a def including inherited ones, own members first.
pub fn all_members<'a>(def: &'a Sym, index: &'a FileIndex, linked: &'a HashMap<String, FileIndex>) -> Vec<&'a Sym> {
    let mut out: Vec<&Sym> = Vec::new();
    let mut current = def;
    let mut ctx = index;
    for _ in 0..16 {
        for c in &current.children {
            if !out.iter().any(|o| o.name == c.name) {
                out.push(c);
            }
        }
        match base_of(current, ctx, linked) {
            Some((next, base)) => {
                current = base;
                ctx = next;
            }
            None => break,
        }
    }
    out
}

/// `<< TODO: ...` style notes inside comments, listed as information.
pub fn annotations(text: &str) -> Vec<Diag> {
    const MARKERS: &[&str] = &["TODO", "FIXME", "BUG", "HACK", "XXX"];
    let Ok((_, comments)) = lex::lex_full(text, FILE_TAG) else { return Vec::new() };
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for c in comments {
        for line in c.line..=c.end_line {
            let Some(source) = lines.get(line - 1) else { continue };
            let chars: Vec<char> = source.chars().collect();
            for marker in MARKERS {
                let m: Vec<char> = marker.chars().collect();
                let mut i = 0;
                while i + m.len() <= chars.len() {
                    let boundary_before = i == 0 || !chars[i - 1].is_alphanumeric();
                    let boundary_after = i + m.len() == chars.len() || !chars[i + m.len()].is_alphanumeric();
                    if chars[i..i + m.len()] == m[..] && boundary_before && boundary_after {
                        let rest: String = chars[i + m.len()..].iter().collect();
                        let rest = rest.trim_start_matches([':', ' ']).trim_end_matches("]>>").trim_end_matches("}>>").trim();
                        out.push(Diag {
                            line,
                            col: i + 1,
                            len: m.len(),
                            severity: Severity::Info,
                            message: if rest.is_empty() { marker.to_string() } else { format!("{}: {}", marker, rest) },
                            help: None,
                        });
                        break;
                    }
                    i += 1;
                }
            }
        }
    }
    out
}

fn native_index(name: &Path) -> FileIndex {
    let key = crate::stdlib::module_alias(&name.to_string_lossy()).map(|(k, _)| k).unwrap_or("");
    let symbols = crate::stdlib::members(key)
        .map(|(_, member, signature, doc)| {
            let constant = crate::stdlib::is_constant(signature);
            Sym {
                name: member.to_string(),
                kind: if constant { SymKind::Variable } else { SymKind::Function },
                pos: Pos::default(),
                end_line: 0,
                signature: signature.to_string(),
                docs: vec![("desc".into(), doc.to_string())],
                exported: true,
                arity: if constant { None } else { crate::stdlib::arity(signature) },
                ty: None,
                children: Vec::new(),
        params: Vec::new(),
                path: name.to_path_buf(),
            }
        })
        .collect();
    FileIndex { path: name.to_path_buf(), symbols, links: Vec::new() }
}

// ==================================================================== index

/// Removes `.` and resolves `..` without touching the file system.
pub fn clean_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Reads `<<{ !tag = "value" }>>` written right above `line`.
pub fn doc_tags(text: &str, line: usize) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    if line < 2 || line - 2 >= lines.len() {
        return Vec::new();
    }
    let mut end = line - 2;
    while end > 0 && lines[end].trim().is_empty() {
        end -= 1;
    }
    // A doc block opens with `<<{` at the start of a line; text inside a
    // `<<` line comment (like an example of the syntax) is not a doc block.
    let is_line_comment = |l: &str| {
        let t = l.trim_start();
        t.starts_with("<<") && !t.starts_with("<<{") && !t.starts_with("<<[")
    };
    if !lines[end].trim_end().ends_with("}>>") || is_line_comment(lines[end]) {
        return Vec::new();
    }
    let mut start = end;
    while !lines[start].trim_start().starts_with("<<{") {
        if start == 0 || is_line_comment(lines[start]) {
            return Vec::new();
        }
        start -= 1;
    }
    let block = lines[start..=end].join("\n");
    let mut tags = Vec::new();
    let chars: Vec<char> = block.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '!' {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[i + 1..j].iter().collect();
            let mut k = j;
            while k < chars.len() && chars[k] != '"' && chars[k] != '!' {
                k += 1;
            }
            if k < chars.len() && chars[k] == '"' {
                let mut value = String::new();
                let mut m = k + 1;
                while m < chars.len() && chars[m] != '"' {
                    if chars[m] == '\\' && m + 1 < chars.len() {
                        m += 1;
                    }
                    value.push(chars[m]);
                    m += 1;
                }
                if !name.is_empty() {
                    tags.push((name, value));
                }
                i = m + 1;
                continue;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    tags
}

fn function_sym(f: &FnDecl, owner: Option<&str>, text: &str, path: &Path, exported: bool, kind: SymKind) -> Sym {
    let params: Vec<String> = f
        .param_types
        .iter()
        .zip(&f.params)
        .map(|(t, n)| format!("{} {}", t.render(), n))
        .collect();
    let qualified = match owner {
        Some(o) => format!("{}.{}", o, f.name),
        None => f.name.clone(),
    };
    Sym {
        name: f.name.clone(),
        kind,
        pos: f.pos,
        end_line: f.end_line,
        signature: format!("{} {}({})", f.ret.render(), qualified, params.join(", ")),
        docs: doc_tags(text, f.pos.line),
        exported,
        arity: Some(f.params.len()),
        ty: Some(f.ret.render()),
        children: Vec::new(),
        params: f.param_types.iter().map(TypeName::render).collect(),
        path: path.to_path_buf(),
    }
}

pub fn build_index(path: &Path, program: &Program, text: &str) -> FileIndex {
    let base_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut symbols = Vec::new();
    let mut links = Vec::new();
    let leaf = |pos: Pos, name: &str, kind: SymKind, signature: String, exported: bool, ty: Option<String>| Sym {
        name: name.to_string(),
        kind,
        pos,
        end_line: pos.line,
        signature,
        docs: doc_tags(text, pos.line),
        exported,
        arity: None,
        ty,
        children: Vec::new(),
        params: Vec::new(),
        path: path.to_path_buf(),
    };
    for item in &program.items {
        match item {
            Item::Link { path: target, alias, line, pos } => {
                let native = target.starts_with('@');
                let target_path = if native { PathBuf::from(target) } else { clean_path(&base_dir.join(target)) };
                let alias_name = alias.clone().unwrap_or_else(|| {
                    if native {
                        crate::stdlib::module_alias(target).map(|(_, a)| a.to_string()).unwrap_or_else(|| "Std".to_string())
                    } else {
                        target_path.file_stem().and_then(|s| s.to_str()).unwrap_or("Module").to_string()
                    }
                });
                let mut sym = leaf(*pos, &alias_name, SymKind::Module, format!("link \"{}\" as {}", target, alias_name), false, None);
                sym.path = target_path.clone();
                symbols.push(sym);
                links.push(Link { alias: alias_name, target: target_path, native, line: *line, pos: *pos });
            }
            Item::Function { decl, exported } => {
                symbols.push(function_sym(decl, None, text, path, *exported, SymKind::Function));
            }
            Item::Def { decl, exported } => {
                let mut sym = leaf(decl.pos, &decl.name, SymKind::Def, String::new(), *exported, decl.base.as_ref().map(TypeName::render));
                sym.end_line = decl.end_line;
                sym.params = decl.type_params.clone();
                let generic = if decl.type_params.is_empty() { String::new() } else { format!("<{}>", decl.type_params.join(", ")) };
                let mut lines = vec![format!("def {}{} {{", decl.name, generic)];
                for f in &decl.fields {
                    lines.push(format!("    {} {}", f.ty.render(), f.name));
                    sym.children.push(leaf(
                        f.pos,
                        &f.name,
                        SymKind::Field,
                        format!("{} {}.{}", f.ty.render(), decl.name, f.name),
                        true,
                        Some(f.ty.render()),
                    ));
                }
                for m in &decl.methods {
                    let method = function_sym(m, Some(&decl.name), text, path, true, SymKind::Method);
                    lines.push(format!("    {}", method.signature.replacen(&format!("{}.", decl.name), "", 1)));
                    sym.children.push(method);
                }
                lines.push("}".into());
                sym.signature = lines.join("\n");
                symbols.push(sym);
            }
            Item::Enum { decl, exported } => {
                let mut sym = leaf(
                    decl.pos,
                    &decl.name,
                    SymKind::Enum,
                    format!("enum class {} {{ {} }}", decl.name, decl.variants.join(", ")),
                    *exported,
                    None,
                );
                for (v, p) in decl.variants.iter().zip(&decl.variant_pos) {
                    sym.children.push(leaf(*p, v, SymKind::Variant, format!("{}::{}", decl.name, v), true, None));
                }
                symbols.push(sym);
            }
            Item::TypeAlias { name, exported, pos } => {
                symbols.push(leaf(*pos, name, SymKind::TypeAlias, format!("def {} = ...", name), *exported, None));
            }
            Item::Global { stmt, exported } => {
                if let StmtKind::Var { name, ty, pos, .. } = &stmt.kind {
                    let ty_text = ty.as_ref().map(TypeName::render).unwrap_or_else(|| "auto".into());
                    symbols.push(leaf(*pos, name, SymKind::Variable, format!("{} {}", ty_text, name), *exported, ty.as_ref().map(TypeName::render)));
                }
            }
            Item::Stmt(_) => {}
        }
    }
    FileIndex { path: path.to_path_buf(), symbols, links }
}

impl FileIndex {
    pub fn find(&self, name: &str) -> Option<&Sym> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Innermost function or method containing `line`, with its def if any.
    pub fn enclosing(&self, line: usize) -> (Option<&Sym>, Option<&Sym>) {
        for s in &self.symbols {
            if s.kind == SymKind::Function && s.pos.line <= line && line <= s.end_line {
                return (Some(s), None);
            }
            if s.kind == SymKind::Def && s.pos.line <= line && line <= s.end_line {
                let method = s.children.iter().find(|m| m.kind == SymKind::Method && m.pos.line <= line && line <= m.end_line);
                return (method, Some(s));
            }
        }
        (None, None)
    }
}

// ==================================================================== checker

struct Checker<'a> {
    a: &'a Analysis,
    index: &'a FileIndex,
    diags: Vec<Diag>,
    scopes: Vec<HashMap<String, Option<String>>>,
    def: Option<&'a Sym>,
}

impl<'a> Checker<'a> {
    fn column_of(&self, line: usize, name: &str) -> usize {
        self.a
            .tokens
            .iter()
            .find(|t| t.line == line && t.kind == Tk::Ident && t.text == name)
            .map(|t| t.column)
            .unwrap_or(1)
    }

    fn error(&mut self, line: usize, name: &str, message: String) {
        self.error_help(line, name, message, None);
    }

    fn error_help(&mut self, line: usize, name: &str, message: String, help: Option<String>) {
        let col = self.column_of(line, name);
        self.diags.push(Diag { line, col, len: name.chars().count().max(1), severity: Severity::Error, message, help });
    }

    /// Names visible here, for "did you mean" suggestions.
    fn visible_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.scopes.iter().flat_map(|s| s.keys().cloned()).collect();
        names.extend(self.index.symbols.iter().map(|s| s.name.clone()));
        names.extend(BUILTINS.iter().map(|(b, _, _)| b.to_string()));
        names.extend(self.a.linked.keys().cloned());
        names
    }

    fn is_local(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|s| s.contains_key(name))
    }

    fn declare(&mut self, name: &str, ty: Option<String>, line: usize) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                let msg = format!("'{}' is already declared in this scope", name);
                self.error(line, name, msg);
                return;
            }
            scope.insert(name.to_string(), ty);
        }
    }

    fn check_program(&mut self, program: &'a Program) {
        // Top-level names may be declared twice by mistake.
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for s in &self.index.symbols {
            if let Some(first) = seen.insert(&s.name, s.pos.line) {
                let msg = format!("'{}' is already declared on line {}", s.name, first);
                self.diags.push(Diag { line: s.pos.line, col: s.pos.col, len: s.name.len(), severity: Severity::Error, message: msg, help: None });
            }
        }
        self.scopes.push(HashMap::new());
        for item in &program.items {
            match item {
                Item::Function { decl, .. } => self.check_function(decl, None),
                Item::Def { decl, .. } => {
                    let def_sym = self.index.find(&decl.name);
                    for f in &decl.fields {
                        if let Some(d) = &f.default {
                            self.expr(d);
                        }
                    }
                    for m in &decl.methods {
                        self.check_function(m, def_sym);
                    }
                }
                Item::Global { stmt, .. } => {
                    if let StmtKind::Var { init, ty, pos, .. } = &stmt.kind {
                        if let Some(e) = init {
                            self.expr(e);
                        }
                        if let Some(t) = ty {
                            self.check_type(t, pos.line);
                            if let (Some(e), true) = (init, crate::stdlib::is_numeric_type(&t.base)) {
                                self.check_typed(t, e, pos.line);
                            }
                        }
                    }
                }
                Item::Stmt(stmt) => self.stmt(stmt),
                _ => {}
            }
        }
        self.scopes.pop();
    }

    fn check_function(&mut self, f: &'a FnDecl, def: Option<&'a Sym>) {
        let saved = self.def;
        self.def = def;
        let mut scope = HashMap::new();
        for (name, ty) in f.params.iter().zip(&f.param_types) {
            scope.insert(name.clone(), Some(ty.render()));
        }
        self.scopes.push(scope);
        self.block(&f.body);
        self.scopes.pop();
        self.def = saved;
    }

    fn block(&mut self, stmts: &'a [Stmt]) {
        self.scopes.push(HashMap::new());
        for s in stmts {
            self.stmt(s);
        }
        self.scopes.pop();
    }

    fn stmt(&mut self, s: &'a Stmt) {
        match &s.kind {
            StmtKind::Var { name, ty, init, pos } => {
                if let Some(e) = init {
                    self.expr(e);
                }
                if let Some(t) = ty {
                    self.check_type(t, pos.line);
                    if let (Some(e), true) = (init, crate::stdlib::is_numeric_type(&t.base)) {
                        self.check_typed(t, e, pos.line);
                    }
                }
                self.declare(name, ty.as_ref().map(TypeName::render), pos.line);
            }
            StmtKind::Expr(e) => self.expr(e),
            StmtKind::Block(b) => self.block(b),
            StmtKind::If(c, then, otherwise) => {
                self.expr(c);
                self.block(then);
                if let Some(o) = otherwise {
                    self.block(o);
                }
            }
            StmtKind::While(c, b) => {
                self.expr(c);
                self.block(b);
            }
            StmtKind::DoWhile(b, c) => {
                self.block(b);
                self.expr(c);
            }
            StmtKind::ForIn { name, iterable, body, .. } => {
                self.expr(iterable);
                let mut scope = HashMap::new();
                scope.insert(name.clone(), None);
                self.scopes.push(scope);
                self.block(body);
                self.scopes.pop();
            }
            StmtKind::Match(subject, arms) => {
                self.expr(subject);
                for arm in arms {
                    for p in arm.patterns.iter().flatten() {
                        self.expr(p);
                    }
                    self.block(&arm.body);
                }
            }
            StmtKind::Guard(c, b) => {
                self.expr(c);
                self.block(b);
            }
            StmtKind::Try { body, binding, handler } => {
                self.block(body);
                let mut scope = HashMap::new();
                if let Some(b) = binding {
                    scope.insert(b.clone(), Some("string".into()));
                }
                self.scopes.push(scope);
                self.block(handler);
                self.scopes.pop();
            }
            StmtKind::Return(Some(e)) => self.expr(e),
            StmtKind::Return(None) | StmtKind::Break | StmtKind::Continue => {}
        }
    }

    /// `(Type value)` and `Type name = literal`: the type must exist and a
    /// literal must fit in a sized number type.
    fn check_typed(&mut self, ty: &TypeName, value: &Expr, line: usize) {
        let name = ty.base.as_str();
        let builtin = matches!(name, "string" | "bool" | "array" | "dictionary") || crate::stdlib::is_numeric_type(name);
        if !builtin && !name.contains('.') && !self.is_local(name) {
            match self.global(name) {
                Some(s) if matches!(s.kind, SymKind::Def | SymKind::Enum | SymKind::TypeAlias) => {}
                Some(s) => {
                    let msg = format!("'{}' is not a type", s.name);
                    self.error(line, name, msg);
                }
                None => {
                    let mut types: Vec<String> = crate::stdlib::NUMERIC_TYPES.iter().map(|t| t.to_string()).collect();
                    types.extend(["string", "bool", "array", "dictionary"].iter().map(|t| t.to_string()));
                    types.extend(
                        self.index.symbols.iter().filter(|s| matches!(s.kind, SymKind::Def | SymKind::Enum)).map(|s| s.name.clone()),
                    );
                    let help = crate::diag::closest(name, types.iter().map(String::as_str)).map(|c| format!("did you mean '{}'?", c));
                    self.error_help(line, name, format!("unknown type '{}'", name), help);
                }
            }
        } else {
            self.check_type(ty, line);
        }
        if crate::stdlib::is_numeric_type(name) {
            let literal = match &value.kind {
                ExprKind::Int(i) => Some(crate::stdlib::SVal::Int(*i)),
                ExprKind::Float(f) => Some(crate::stdlib::SVal::Float(*f)),
                ExprKind::Unary(op, inner) if op == "-" => match inner.kind {
                    ExprKind::Int(i) => Some(crate::stdlib::SVal::Int(-i)),
                    ExprKind::Float(f) => Some(crate::stdlib::SVal::Float(-f)),
                    _ => None,
                },
                _ => None,
            };
            if let Some(v) = literal {
                if let Err(message) = crate::stdlib::coerce(name, &v, true) {
                    let help = if message.contains("whole number") {
                        Some(format!("use a whole number, or a float type: (f32 {})", v.text()))
                    } else {
                        Some("use a wider type such as i32, i64 or f64".to_string())
                    };
                    self.error_help(line, name, message, help);
                }
            }
        }
    }

    /// Checks that a qualified type like `Ast.Node` refers to something real.
    fn check_type(&mut self, t: &TypeName, line: usize) {
        if let Some((alias, member)) = t.base.split_once('.') {
            if self.is_local(alias) {
                return;
            }
            if let Some(linked) = self.a.linked.get(alias) {
                match linked.find(member) {
                    None => {
                        let msg = format!("module {} has no type '{}'", alias, member);
                        self.error(line, member, msg);
                    }
                    Some(s) if !s.exported => {
                        let msg = format!("'{}' is not exported by {}", member, alias);
                        self.error(line, member, msg);
                    }
                    _ => {}
                }
            }
        }
        for arg in &t.args {
            self.check_type(arg, line);
        }
    }

    fn global(&self, name: &str) -> Option<&'a Sym> {
        self.index.find(name)
    }

    fn linked_member(&mut self, alias: &str, member: &str, line: usize) -> Option<&'a Sym> {
        let linked = self.a.linked.get(alias)?;
        match linked.find(member) {
            None => {
                let msg = format!("module {} has no member '{}'", alias, member);
                let help = crate::diag::closest(member, linked.symbols.iter().filter(|s| s.exported).map(|s| s.name.as_str()))
                    .map(|c| format!("did you mean '{}.{}'?", alias, c));
                self.error_help(line, member, msg, help);
                None
            }
            Some(s) if !s.exported => {
                let msg = format!("'{}' is not exported by {} (add 'export' to its declaration)", member, alias);
                self.error(line, member, msg);
                None
            }
            Some(s) => Some(s),
        }
    }

    fn is_module_alias(&self, name: &str) -> bool {
        !self.is_local(name) && self.a.linked.contains_key(name)
    }

    fn this_member(&mut self, name: &str, line: usize) -> Option<&'a Sym> {
        match self.def {
            None => {
                self.error(line, name, "'@' members can only be used inside a def method".into());
                None
            }
            Some(def) => match member_in_chain(def, self.index, &self.a.linked, name) {
                Some(m) => Some(m),
                None => {
                    let msg = format!("def {} has no member '{}'", def.name, name);
                    let members = all_members(def, self.index, &self.a.linked);
                    let help = crate::diag::closest(name, members.iter().map(|c| c.name.as_str()))
                        .map(|c| format!("did you mean '@{}'?", c))
                        .or_else(|| Some(format!("declare it in def {}, for example: number {}", def.name, name)));
                    self.error_help(line, name, msg, help);
                    None
                }
            },
        }
    }

    /// Resolves an expression to the def it names, for `Type { ... }`.
    fn def_of(&mut self, e: &Expr) -> Option<&'a Sym> {
        match &e.kind {
            ExprKind::Ident(n) if !self.is_local(n) => self.global(n).filter(|s| s.kind == SymKind::Def),
            ExprKind::Member(base, member) => match &base.kind {
                ExprKind::Ident(alias) if self.is_module_alias(alias) => {
                    self.a.linked.get(alias.as_str())?.find(member).filter(|s| s.kind == SymKind::Def)
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn expr(&mut self, e: &'a Expr) {
        match &e.kind {
            ExprKind::Ident(name) => {
                if !self.is_local(name)
                    && self.global(name).is_none()
                    && !BUILTINS.iter().any(|(b, _, _)| b == name)
                    && !self.a.linked.contains_key(name)
                {
                    let msg = format!("'{}' is not defined", name);
                    let names = self.visible_names();
                    let help = crate::diag::closest(name, names.iter().map(String::as_str))
                        .map(|c| format!("did you mean '{}'?", c))
                        .or_else(|| Some(format!("declare it before using it, for example: auto {} = ...", name)));
                    self.error_help(e.line, name, msg, help);
                }
            }
            ExprKind::This => {
                if self.def.is_none() {
                    self.diags.push(Diag {
                        line: e.line,
                        col: self.column_of(e.line, "this"),
                        len: 4,
                        severity: Severity::Error,
                        message: "'@this' can only be used inside a def method".into(),
                        help: None,
                    });
                }
            }
            ExprKind::Member(base, member) => match &base.kind {
                ExprKind::Ident(alias) if self.is_module_alias(alias) => {
                    self.linked_member(alias, member, e.line);
                }
                ExprKind::This => {
                    self.this_member(member, e.line);
                }
                _ => self.expr(base),
            },
            ExprKind::Scope(base, variant) => {
                let enum_sym = match &base.kind {
                    ExprKind::Ident(n) if !self.is_local(n) => self.global(n),
                    ExprKind::Member(inner, n) => match &inner.kind {
                        ExprKind::Ident(alias) if self.is_module_alias(alias) => self.linked_member(alias, n, e.line),
                        _ => None,
                    },
                    _ => None,
                };
                match enum_sym {
                    Some(s) if s.kind == SymKind::Enum => {
                        if !s.children.iter().any(|v| &v.name == variant) {
                            let msg = format!("enum class {} has no member '{}'", s.name, variant);
                            self.error(e.line, variant, msg);
                        }
                    }
                    Some(s) => {
                        let msg = format!("'::' needs an enum class, but '{}' is not one", s.name);
                        self.error(e.line, &s.name.clone(), msg);
                    }
                    None => self.expr(base),
                }
            }
            ExprKind::Call(callee, args) => {
                let target: Option<&Sym> = match &callee.kind {
                    ExprKind::Ident(n) if !self.is_local(n) => {
                        self.expr(callee);
                        self.global(n)
                    }
                    ExprKind::Member(base, m) => match &base.kind {
                        ExprKind::Ident(alias) if self.is_module_alias(alias) => self.linked_member(alias, m, e.line),
                        ExprKind::This => self.this_member(m, e.line),
                        _ => {
                            self.expr(callee);
                            None
                        }
                    },
                    _ => {
                        self.expr(callee);
                        None
                    }
                };
                if let Some(s) = target {
                    if let Some(n) = s.arity {
                        if n != args.len() {
                            let msg = format!("{} expects {} argument(s), got {}", s.name, n, args.len());
                            self.error(e.line, &s.name.clone(), msg);
                        }
                    } else if s.kind == SymKind::Def {
                        let msg = format!("use '{} {{ Field = value }}' to create a {}", s.name, s.name);
                        self.error(e.line, &s.name.clone(), msg);
                    }
                }
                for a in args {
                    self.expr(a);
                }
            }
            ExprKind::New(type_path, fields) => {
                match self.def_of(type_path) {
                    Some(def) => {
                        for (name, _) in fields {
                            if !all_members(def, self.index, &self.a.linked).iter().any(|c| c.kind == SymKind::Field && &c.name == name) {
                                let msg = format!("def {} has no field '{}'", def.name, name);
                                self.error(e.line, name, msg);
                            }
                        }
                    }
                    None => self.expr(type_path),
                }
                for (_, v) in fields {
                    self.expr(v);
                }
            }
            ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Assign(_, a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Unary(_, a) | ExprKind::Postfix(_, a) => self.expr(a),
            ExprKind::Ternary(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            ExprKind::Lambda(decl) => {
                let mut scope = HashMap::new();
                for (name, ty) in decl.params.iter().zip(&decl.param_types) {
                    scope.insert(name.clone(), Some(ty.render()));
                }
                self.scopes.push(scope);
                self.block(&decl.body);
                self.scopes.pop();
            }
            ExprKind::Typed(ty, inner) => {
                self.expr(inner);
                self.check_typed(ty, inner, e.line);
            }
            ExprKind::ArrayLit(items) | ExprKind::TypedCollection(_, items) => {
                for i in items {
                    self.expr(i);
                }
            }
            ExprKind::DictLit(entries) => {
                for (k, v) in entries {
                    if let DictKey::Expr(k) = k {
                        self.expr(k);
                    }
                    self.expr(v);
                }
            }
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_) | ExprKind::Bool(_) | ExprKind::Null => {}
        }
    }
}

// ==================================================================== locals

/// Parameters and variables visible in the function around `line`.
pub fn locals_at(program: &Program, index: &FileIndex, line: usize) -> Vec<Sym> {
    let (function, def) = index.enclosing(line);
    let mut out = Vec::new();
    let decl: Option<&FnDecl> = function.and_then(|f| {
        program.items.iter().find_map(|item| match item {
            Item::Function { decl, .. } if decl.pos == f.pos => Some(decl.as_ref()),
            Item::Def { decl, .. } if def.map(|d| d.pos == decl.pos).unwrap_or(false) => {
                decl.methods.iter().find(|m| m.pos == f.pos).map(|m| m.as_ref())
            }
            _ => None,
        })
    });
    let make = |name: &str, kind: SymKind, pos: Pos, ty: Option<String>| Sym {
        name: name.to_string(),
        kind,
        pos,
        end_line: pos.line,
        signature: format!("{} {}", ty.clone().unwrap_or_else(|| "auto".into()), name),
        docs: Vec::new(),
        exported: false,
        arity: None,
        ty,
        children: Vec::new(),
        params: Vec::new(),
        path: index.path.clone(),
    };
    let stmts: &[Stmt] = match decl {
        Some(f) => {
            for (n, t) in f.params.iter().zip(&f.param_types) {
                out.push(make(n, SymKind::Param, f.pos, Some(t.render())));
            }
            &f.body
        }
        None => &[],
    };
    fn walk(stmts: &[Stmt], line: usize, out: &mut Vec<Sym>, make: &dyn Fn(&str, SymKind, Pos, Option<String>) -> Sym) {
        for s in stmts {
            if s.line > line {
                break;
            }
            match &s.kind {
                StmtKind::Var { name, ty, pos, .. } => out.push(make(name, SymKind::Local, *pos, ty.as_ref().map(TypeName::render))),
                StmtKind::ForIn { name, ty, body, pos, .. } => {
                    out.push(make(name, SymKind::Local, *pos, ty.as_ref().map(TypeName::render)));
                    walk(body, line, out, make);
                }
                StmtKind::Block(b) | StmtKind::While(_, b) | StmtKind::DoWhile(b, _) | StmtKind::Guard(_, b) => walk(b, line, out, make),
                StmtKind::Match(_, arms) => {
                    for arm in arms {
                        walk(&arm.body, line, out, make);
                    }
                }
                StmtKind::If(_, a, b) => {
                    walk(a, line, out, make);
                    if let Some(b) = b {
                        walk(b, line, out, make);
                    }
                }
                StmtKind::Try { body, handler, binding } => {
                    walk(body, line, out, make);
                    if let Some(b) = binding {
                        out.push(make(b, SymKind::Local, Pos { line: s.line, col: 1 }, Some("string".into())));
                    }
                    walk(handler, line, out, make);
                }
                _ => {}
            }
        }
    }
    walk(stmts, line, &mut out, &make);
    if decl.is_none() {
        // Top-level statements: globals are already in the index.
        let top: Vec<Stmt> = program
            .items
            .iter()
            .filter_map(|i| if let Item::Stmt(s) = i { Some(s.clone()) } else { None })
            .collect();
        walk(&top, line, &mut out, &make);
    }
    out
}

/// The def a type name refers to, looking through linked modules.
pub fn def_for_type<'a>(analysis: &'a Analysis, ty: &str) -> Option<&'a Sym> {
    let base = ty.split('<').next().unwrap_or(ty);
    match base.split_once('.') {
        Some((alias, name)) => analysis.linked.get(alias)?.find(name).filter(|s| s.kind == SymKind::Def),
        None => analysis.index.as_ref()?.find(base).filter(|s| s.kind == SymKind::Def),
    }
}

/// Resolves the identifier token at `token_index` to its declaration.
pub fn resolve(analysis: &Analysis, token_index: usize) -> Option<Sym> {
    let index = analysis.index.as_ref()?;
    let program = analysis.program.as_ref()?;
    let toks = &analysis.tokens;
    let tok = toks.get(token_index)?;
    if tok.kind != Tk::Ident {
        return None;
    }
    let name = tok.text.as_str();
    let sym_at = |i: usize, s: &str| i < toks.len() && toks[i].kind == Tk::Sym && toks[i].text == s;
    let word_at = |i: usize| if i < toks.len() && toks[i].kind == Tk::Ident { Some(toks[i].text.as_str()) } else { None };
    let line = tok.line;
    let (_, def) = index.enclosing(line);

    if token_index >= 1 && sym_at(token_index - 1, "@") {
        return def?.children.iter().find(|c| c.name == name).cloned();
    }
    if token_index >= 2 && sym_at(token_index - 1, ".") {
        let owner = word_at(token_index - 2)?;
        if owner == "this" && token_index >= 3 && sym_at(token_index - 3, "@") {
            return def?.children.iter().find(|c| c.name == name).cloned();
        }
        let local = locals_at(program, index, line).into_iter().rev().find(|l| l.name == owner);
        if local.is_none() {
            if let Some(linked) = analysis.linked.get(owner) {
                return linked.find(name).cloned();
            }
        }
        let ty = match local {
            Some(l) => l.ty,
            None => index.find(owner).and_then(|g| g.ty.clone()),
        }?;
        return def_for_type(analysis, &ty)?.children.iter().find(|c| c.name == name).cloned();
    }
    if token_index >= 2 && sym_at(token_index - 1, "::") {
        let enum_name = word_at(token_index - 2)?;
        let enum_sym = if token_index >= 4 && sym_at(token_index - 3, ".") {
            analysis.linked.get(word_at(token_index - 4)?)?.find(enum_name)?
        } else {
            index.find(enum_name)?
        };
        return enum_sym.children.iter().find(|c| c.name == name).cloned();
    }
    if let Some(l) = locals_at(program, index, line).into_iter().rev().find(|l| l.name == name) {
        return Some(l);
    }
    if let Some(s) = index.find(name) {
        return Some(s.clone());
    }
    BUILTINS.iter().find(|(b, _, _)| *b == name).map(|(b, sig, doc)| Sym {
        name: b.to_string(),
        kind: SymKind::Builtin,
        pos: Pos::default(),
        end_line: 0,
        signature: sig.to_string(),
        docs: vec![("desc".into(), doc.to_string())],
        exported: true,
        arity: None,
        ty: None,
        children: Vec::new(),
        params: Vec::new(),
        path: PathBuf::new(),
    })
}

/// Token under a 1-based line and 1-based column.
pub fn token_at(tokens: &[Token], line: usize, col: usize) -> Option<usize> {
    let width = |t: &Token| t.text.chars().count().max(1);
    // Prefer the token that contains the cursor; else the one that ends right before it.
    tokens
        .iter()
        .position(|t| t.line == line && t.kind != Tk::Eof && t.column <= col && col < t.column + width(t))
        .or_else(|| {
            tokens
                .iter()
                .position(|t| t.line == line && t.kind == Tk::Ident && col == t.column + width(t))
        })
}
