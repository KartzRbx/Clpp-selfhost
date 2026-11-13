//! Language server over stdio (JSON-RPC with Content-Length framing).
//!
//! `clpp-boot lsp` provides diagnostics, document symbols, hover with doc
//! comments, go to definition and completion.

use crate::analysis::{self, Analysis, Sym, SymKind};
use crate::json::{self, Json};
use crate::lex::Tk;
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};

pub fn uri_to_path(uri: &str) -> PathBuf {
    let rest = uri.strip_prefix("file://").unwrap_or(uri);
    let mut decoded = Vec::new();
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&rest[i + 1..i + 3], 16) {
                decoded.push(v);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    let mut text = String::from_utf8_lossy(&decoded).to_string();
    // file:///c:/x -> c:/x on Windows.
    if text.len() > 2 && text.starts_with('/') && text.as_bytes()[2] == b':' {
        text.remove(0);
    }
    PathBuf::from(text)
}

pub fn path_to_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text).to_string();
    let mut out = String::from("file://");
    if !text.starts_with('/') {
        out.push('/');
    }
    for c in text.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '%' => out.push_str("%25"),
            '?' => out.push_str("%3F"),
            c => out.push(c),
        }
    }
    out
}

/// Normalizes paths so that a linked file and an open document compare equal.
fn key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in text.split('/') {
        match part {
            "." | "" if !parts.is_empty() => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/").to_lowercase()
}

pub struct Server {
    docs: HashMap<String, (String, String)>,
    /// Last analysis of each document that parsed, used while the user types.
    good: std::cell::RefCell<HashMap<String, Analysis>>,
    shutting_down: bool,
}

fn range(line: usize, col: usize, len: usize) -> Json {
    let l = line.saturating_sub(1) as f64;
    let c = col.saturating_sub(1) as f64;
    Json::obj(vec![
        ("start", Json::obj(vec![("line", Json::num(l)), ("character", Json::num(c))])),
        ("end", Json::obj(vec![("line", Json::num(l)), ("character", Json::num(c + len as f64))])),
    ])
}

fn span(start_line: usize, end_line: usize) -> Json {
    Json::obj(vec![
        ("start", Json::obj(vec![("line", Json::num(start_line.saturating_sub(1) as f64)), ("character", Json::num(0))])),
        ("end", Json::obj(vec![("line", Json::num(end_line.saturating_sub(1) as f64)), ("character", Json::num(1000))])),
    ])
}

fn symbol_kind(kind: SymKind) -> f64 {
    match kind {
        SymKind::Function => 12.0,
        SymKind::Method => 6.0,
        SymKind::Field => 8.0,
        SymKind::Def => 23.0,
        SymKind::Enum => 10.0,
        SymKind::Variant => 22.0,
        SymKind::Variable | SymKind::Local | SymKind::Param => 13.0,
        SymKind::TypeAlias => 26.0,
        SymKind::Module => 2.0,
        SymKind::Builtin => 12.0,
    }
}

fn completion_kind(kind: SymKind) -> f64 {
    match kind {
        SymKind::Function | SymKind::Builtin => 3.0,
        SymKind::Method => 2.0,
        SymKind::Field => 5.0,
        SymKind::Def => 22.0,
        SymKind::Enum => 13.0,
        SymKind::Variant => 20.0,
        SymKind::Variable | SymKind::Local | SymKind::Param => 6.0,
        SymKind::TypeAlias => 7.0,
        SymKind::Module => 9.0,
    }
}

pub fn hover_markdown(sym: &Sym) -> String {
    let mut out = format!("```clpp\n{}\n```", sym.signature);
    let mut tags: Vec<(String, String)> = sym.docs.iter().filter(|(n, _)| n != "name").cloned().collect();
    tags.sort_by_key(|(n, _)| match n.as_str() {
        "deprecated" => 0,
        "desc" => 1,
        "params" => 2,
        "return" => 3,
        "example" => 4,
        _ => 5,
    });
    for (name, value) in tags {
        let label = match name.as_str() {
            "desc" => String::new(),
            "params" => "**Parameters:** ".into(),
            "return" => "**Returns:** ".into(),
            "example" => "**Example:** ".into(),
            "deprecated" => "**Deprecated:** ".into(),
            "see" => "**See also:** ".into(),
            other => format!("**{}:** ", other),
        };
        out.push_str(&format!("\n\n{}{}", label, value));
    }
    let kind = match sym.kind {
        SymKind::Local => Some("local variable"),
        SymKind::Param => Some("parameter"),
        SymKind::Builtin => Some("built-in"),
        _ => None,
    };
    if let Some(k) = kind {
        out.push_str(&format!("\n\n_{}_", k));
    }
    out
}

impl Server {
    pub fn new() -> Server {
        Server { docs: HashMap::new(), good: std::cell::RefCell::new(HashMap::new()), shutting_down: false }
    }

    /// Analysis for navigation: the current text when it parses, else the
    /// last version that did.
    fn usable(&self, uri: &str) -> Option<Analysis> {
        let current = self.analyze_uri(uri)?;
        if current.index.is_some() {
            return Some(current);
        }
        let k = key(&uri_to_path(uri));
        self.good.borrow().get(&k).cloned().or(Some(current))
    }

    /// Analysis for completion: the line being typed is usually incomplete,
    /// so analyze the document without it.
    fn for_completion(&self, uri: &str, line: usize) -> Option<Analysis> {
        let path = uri_to_path(uri);
        let (_, text) = self.docs.get(&key(&path))?;
        let blanked: Vec<&str> = text.split('\n').enumerate().map(|(i, l)| if i + 1 == line { "" } else { l }).collect();
        let loader = |p: &Path| self.load(p);
        let a = analysis::analyze(&path, &blanked.join("\n"), &loader);
        if a.index.is_some() {
            return Some(a);
        }
        self.usable(uri)
    }

    fn load(&self, path: &Path) -> Option<String> {
        if let Some((_, text)) = self.docs.get(&key(path)) {
            return Some(text.clone());
        }
        std::fs::read_to_string(path).ok()
    }

    fn analyze_uri(&self, uri: &str) -> Option<Analysis> {
        let path = uri_to_path(uri);
        let (_, text) = self.docs.get(&key(&path))?;
        let loader = |p: &Path| self.load(p);
        Some(analysis::analyze(&path, text, &loader))
    }

    fn diagnostics(&self, uri: &str) -> Json {
        let analysis = self.analyze_uri(uri);
        if let Some(a) = &analysis {
            if a.index.is_some() {
                self.good.borrow_mut().insert(key(&uri_to_path(uri)), a.clone());
            }
        }
        let items = match analysis {
            Some(a) => a
                .diags
                .iter()
                .map(|d| {
                    Json::obj(vec![
                        ("range", range(d.line, d.col, d.len)),
                        ("severity", Json::num(d.severity as i32)),
                        ("source", Json::str("clpp")),
                        ("message", Json::str(d.message.clone())),
                    ])
                })
                .collect(),
            None => Vec::new(),
        };
        Json::obj(vec![
            ("jsonrpc", Json::str("2.0")),
            ("method", Json::str("textDocument/publishDiagnostics")),
            ("params", Json::obj(vec![("uri", Json::str(uri)), ("diagnostics", Json::Arr(items))])),
        ])
    }

    fn position(params: &Json) -> (String, usize, usize) {
        let uri = params.get("textDocument").get("uri").as_str().unwrap_or("").to_string();
        let line = params.get("position").get("line").as_usize().unwrap_or(0) + 1;
        let col = params.get("position").get("character").as_usize().unwrap_or(0) + 1;
        (uri, line, col)
    }

    fn location(sym: &Sym, fallback: &Path) -> Json {
        let path = if sym.path.as_os_str().is_empty() { fallback } else { &sym.path };
        let (line, col) = if sym.pos.line == 0 { (1, 1) } else { (sym.pos.line, sym.pos.col) };
        let len = if sym.kind == SymKind::Module { 0 } else { sym.name.chars().count() };
        Json::obj(vec![("uri", Json::str(path_to_uri(path))), ("range", range(line, col, len))])
    }

    fn hover(&self, params: &Json) -> Json {
        let (uri, line, col) = Self::position(params);
        let Some(a) = self.usable(&uri) else { return Json::Null };
        let Some(i) = analysis::token_at(&a.tokens, line, col) else { return Json::Null };
        match analysis::resolve(&a, i) {
            Some(sym) => Json::obj(vec![(
                "contents",
                Json::obj(vec![("kind", Json::str("markdown")), ("value", Json::str(hover_markdown(&sym)))]),
            )]),
            None => Json::Null,
        }
    }

    fn definition(&self, params: &Json) -> Json {
        let (uri, line, col) = Self::position(params);
        let Some(a) = self.usable(&uri) else { return Json::Null };
        let Some(i) = analysis::token_at(&a.tokens, line, col) else { return Json::Null };
        // `link "./file.clp"`: jump to the file.
        if a.tokens[i].kind == Tk::Str {
            if let Some(index) = &a.index {
                if let Some(link) = index.links.iter().find(|l| l.line == line && !l.native) {
                    return Json::obj(vec![("uri", Json::str(path_to_uri(&link.target))), ("range", range(1, 1, 0))]);
                }
            }
            return Json::Null;
        }
        match analysis::resolve(&a, i) {
            Some(sym) if sym.kind != SymKind::Builtin && !sym.path.to_string_lossy().starts_with('@') => {
                Self::location(&sym, &a.path)
            }
            _ => Json::Null,
        }
    }

    fn symbols(&self, params: &Json) -> Json {
        let uri = params.get("textDocument").get("uri").as_str().unwrap_or("").to_string();
        let Some(a) = self.usable(&uri) else { return Json::Arr(Vec::new()) };
        let Some(index) = &a.index else { return Json::Arr(Vec::new()) };
        fn convert(s: &Sym) -> Json {
            let end = s.end_line.max(s.pos.line);
            Json::obj(vec![
                ("name", Json::str(s.name.clone())),
                ("detail", Json::str(s.signature.lines().next().unwrap_or("").to_string())),
                ("kind", Json::num(symbol_kind(s.kind))),
                ("range", span(s.pos.line, end)),
                ("selectionRange", range(s.pos.line, s.pos.col, s.name.chars().count())),
                ("children", Json::Arr(s.children.iter().map(convert).collect())),
            ])
        }
        Json::Arr(index.symbols.iter().filter(|s| s.pos.line > 0).map(convert).collect())
    }

    fn completion(&self, params: &Json) -> Json {
        let (uri, line, col) = Self::position(params);
        let path = uri_to_path(&uri);
        let Some((_, text)) = self.docs.get(&key(&path)) else { return Json::Arr(Vec::new()) };
        let Some(a) = self.for_completion(&uri, line) else { return Json::Arr(Vec::new()) };
        let line_text: String = text.lines().nth(line - 1).unwrap_or("").chars().take(col - 1).collect();
        // Drop the partial word being typed.
        let before = line_text.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_');

        let item = |s: &Sym| {
            let mut pairs = vec![
                ("label", Json::str(s.name.clone())),
                ("kind", Json::num(completion_kind(s.kind))),
                ("detail", Json::str(s.signature.lines().next().unwrap_or("").to_string())),
            ];
            if let Some((_, d)) = s.docs.iter().find(|(n, _)| n == "desc") {
                pairs.push(("documentation", Json::str(d.clone())));
            }
            Json::obj(pairs)
        };
        let mut items: Vec<Json> = Vec::new();

        // The analysis may be from a file that does not parse right now.
        let index = a.index.as_ref();
        let def = index.and_then(|i| i.enclosing(line).1);

        if before.ends_with('@') || before.ends_with("@this.") {
            if let Some(d) = def {
                items.extend(d.children.iter().map(item));
            }
            if before.ends_with('@') {
                items.push(Json::obj(vec![("label", Json::str("this")), ("kind", Json::num(14))]));
            }
            return Json::Arr(items);
        }
        if let Some(owner_text) = before.strip_suffix("::") {
            let owner: Vec<&str> = owner_text.rsplit(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')).next().unwrap_or("").split('.').collect();
            let enum_sym = match owner.as_slice() {
                [alias, name] => a.linked.get(*alias).and_then(|l| l.find(name)),
                [name] => index.and_then(|i| i.find(name)),
                _ => None,
            };
            if let Some(e) = enum_sym {
                items.extend(e.children.iter().map(item));
            }
            return Json::Arr(items);
        }
        if let Some(owner_text) = before.strip_suffix('.') {
            let owner = owner_text.rsplit(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("");
            if let Some(linked) = a.linked.get(owner) {
                items.extend(linked.symbols.iter().filter(|s| s.exported).map(item));
                return Json::Arr(items);
            }
            if let (Some(index), Some(program)) = (index, a.program.as_ref()) {
                let ty = analysis::locals_at(program, index, line)
                    .into_iter()
                    .rev()
                    .find(|l| l.name == owner)
                    .and_then(|l| l.ty)
                    .or_else(|| index.find(owner).and_then(|g| g.ty.clone()));
                if let Some(d) = ty.and_then(|t| analysis::def_for_type(&a, &t)) {
                    items.extend(d.children.iter().map(item));
                }
            }
            return Json::Arr(items);
        }

        for k in analysis::KEYWORDS {
            items.push(Json::obj(vec![("label", Json::str(*k)), ("kind", Json::num(14))]));
        }
        for (name, sig, doc) in analysis::BUILTINS {
            items.push(Json::obj(vec![
                ("label", Json::str(*name)),
                ("kind", Json::num(3)),
                ("detail", Json::str(*sig)),
                ("documentation", Json::str(*doc)),
            ]));
        }
        if let Some(index) = index {
            items.extend(index.symbols.iter().map(item));
            if let Some(program) = a.program.as_ref() {
                let mut seen = std::collections::HashSet::new();
                for l in analysis::locals_at(program, index, line).into_iter().rev() {
                    if seen.insert(l.name.clone()) {
                        items.push(item(&l));
                    }
                }
            }
        }
        Json::Arr(items)
    }

    /// Format Document: one edit that replaces the whole text, or none when
    /// the file is already formatted or does not parse.
    fn formatting(&self, params: &Json) -> Json {
        let uri = params.get("textDocument").get("uri").as_str().unwrap_or("").to_string();
        let Some((_, text)) = self.docs.get(&key(&uri_to_path(&uri))) else { return Json::Arr(Vec::new()) };
        let tab = params.get("options").get("tabSize").as_usize().unwrap_or(4).max(1);
        let spaces = !matches!(params.get("options").get("insertSpaces"), Json::Bool(false));
        let options = crate::fmt::Options { indent: if spaces { " ".repeat(tab) } else { "\t".into() } };
        match crate::fmt::format(text, &options) {
            Ok(formatted) if &formatted != text => {
                let lines = text.split('\n').count();
                Json::Arr(vec![Json::obj(vec![
                    (
                        "range",
                        Json::obj(vec![
                            ("start", Json::obj(vec![("line", Json::num(0)), ("character", Json::num(0))])),
                            ("end", Json::obj(vec![("line", Json::num(lines as f64)), ("character", Json::num(0))])),
                        ]),
                    ),
                    ("newText", Json::str(formatted)),
                ])])
            }
            _ => Json::Arr(Vec::new()),
        }
    }

    /// Handles one message; returns the messages to send back.
    pub fn handle(&mut self, msg: &Json) -> (Vec<Json>, bool) {
        let method = msg.get("method").as_str().unwrap_or("");
        let id = msg.get("id").clone();
        let params = msg.get("params");
        let mut out = Vec::new();
        let reply = |result: Json| {
            Json::obj(vec![("jsonrpc", Json::str("2.0")), ("id", id.clone()), ("result", result)])
        };
        match method {
            "initialize" => out.push(reply(Json::obj(vec![
                (
                    "capabilities",
                    Json::obj(vec![
                        ("textDocumentSync", Json::num(1)),
                        ("hoverProvider", Json::Bool(true)),
                        ("definitionProvider", Json::Bool(true)),
                        ("documentSymbolProvider", Json::Bool(true)),
                        ("documentFormattingProvider", Json::Bool(true)),
                        (
                            "completionProvider",
                            Json::obj(vec![(
                                "triggerCharacters",
                                Json::Arr(vec![Json::str("."), Json::str("@"), Json::str(":")]),
                            )]),
                        ),
                    ]),
                ),
                ("serverInfo", Json::obj(vec![("name", Json::str("clpp-boot")), ("version", Json::str(env!("CARGO_PKG_VERSION")))])),
            ]))),
            "shutdown" => {
                self.shutting_down = true;
                out.push(reply(Json::Null));
            }
            "exit" => return (out, true),
            "textDocument/didOpen" => {
                let doc = params.get("textDocument");
                let uri = doc.get("uri").as_str().unwrap_or("").to_string();
                let text = doc.get("text").as_str().unwrap_or("").to_string();
                self.docs.insert(key(&uri_to_path(&uri)), (uri.clone(), text));
                out.extend(self.refresh_all());
            }
            "textDocument/didChange" => {
                let uri = params.get("textDocument").get("uri").as_str().unwrap_or("").to_string();
                if let Some(text) = params.get("contentChanges").at(0).get("text").as_str() {
                    self.docs.insert(key(&uri_to_path(&uri)), (uri.clone(), text.to_string()));
                }
                // A change can fix or break files that link this one.
                out.extend(self.refresh_all());
            }
            "textDocument/didClose" => {
                let uri = params.get("textDocument").get("uri").as_str().unwrap_or("").to_string();
                self.docs.remove(&key(&uri_to_path(&uri)));
                out.push(Json::obj(vec![
                    ("jsonrpc", Json::str("2.0")),
                    ("method", Json::str("textDocument/publishDiagnostics")),
                    ("params", Json::obj(vec![("uri", Json::str(uri)), ("diagnostics", Json::Arr(Vec::new()))])),
                ]));
            }
            "textDocument/hover" => out.push(reply(self.hover(params))),
            "textDocument/definition" => out.push(reply(self.definition(params))),
            "textDocument/documentSymbol" => out.push(reply(self.symbols(params))),
            "textDocument/completion" => out.push(reply(self.completion(params))),
            "textDocument/formatting" => out.push(reply(self.formatting(params))),
            _ => {
                if !id.is_null() {
                    out.push(Json::obj(vec![
                        ("jsonrpc", Json::str("2.0")),
                        ("id", id.clone()),
                        (
                            "error",
                            Json::obj(vec![("code", Json::num(-32601)), ("message", Json::str(format!("method not found: {}", method)))]),
                        ),
                    ]));
                }
            }
        }
        (out, false)
    }

    fn refresh_all(&self) -> Vec<Json> {
        let uris: Vec<String> = self.docs.values().map(|(u, _)| u.clone()).collect();
        uris.iter().map(|u| self.diagnostics(u)).collect()
    }
}

fn read_message(input: &mut impl BufRead) -> Option<Json> {
    let mut length = None;
    loop {
        let mut header = String::new();
        if input.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some(v) = header.strip_prefix("Content-Length:") {
            length = v.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0u8; length?];
    input.read_exact(&mut body).ok()?;
    json::parse(&String::from_utf8_lossy(&body)).ok()
}

fn write_message(output: &mut impl Write, msg: &Json) {
    let body = msg.to_text();
    let _ = write!(output, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = output.flush();
}

pub fn run() -> i32 {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let mut server = Server::new();
    while let Some(msg) = read_message(&mut input) {
        let (replies, exit) = server.handle(&msg);
        for r in &replies {
            write_message(&mut output, r);
        }
        if exit {
            return if server.shutting_down { 0 } else { 1 };
        }
    }
    0
}
