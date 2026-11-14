//! Static type checking for `clpp-boot check` and the language server.
//!
//! Gradual: an expression whose type cannot be worked out is `?` and never
//! causes an error, so the checker only reports mistakes it is sure about.
//!
//! Types are strings. Defs and enums are written `Name@path` so the same type
//! reached through different `link` aliases compares equal.

use crate::analysis::{Analysis, Diag, FileIndex, Severity, Sym, SymKind};
use crate::ast::*;
use crate::lex::Tk;
use std::collections::HashMap;

const UNKNOWN: &str = "?";
const INTS: &[&str] = &["number", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];
const FLOATS: &[&str] = &["float", "f16", "f32", "f64"];

fn is_int(t: &str) -> bool {
    INTS.contains(&t)
}
fn is_float(t: &str) -> bool {
    FLOATS.contains(&t)
}
fn is_numeric(t: &str) -> bool {
    is_int(t) || is_float(t)
}

/// Readable form: `Node@c:/x/ast.clp` -> `Node`.
pub fn show(t: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for c in t.chars() {
        match c {
            '@' => skipping = true,
            '<' | '>' | ',' => {
                skipping = false;
                out.push(c);
                if c == ',' {
                    out.push(' ');
                }
            }
            _ if !skipping => out.push(c),
            _ => {}
        }
    }
    out
}

fn path_key(p: &std::path::Path) -> String {
    crate::analysis::clean_path(p).to_string_lossy().replace('\\', "/").to_lowercase()
}

/// Splits `array<x, y>` into ("array", ["x", "y"]), respecting nesting.
fn split_type(t: &str) -> (String, Vec<String>) {
    let Some(open) = t.find('<') else { return (t.to_string(), Vec::new()) };
    let inner = &t[open + 1..t.len().saturating_sub(1)];
    let mut args = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '<' => {
                depth += 1;
                current.push(c);
            }
            '>' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    (t[..open].to_string(), args)
}

/// Canonical type of a type written in the file `ctx`.
fn canon(written: &str, ctx: &FileIndex) -> String {
    let (base, args) = split_type(written.trim());
    let args: Vec<String> = args.iter().map(|a| canon(a, ctx)).collect();
    let base = match base.as_str() {
        _ if base.contains('@') => base, // already canonical (substituted type argument)
        "number" | "float" | "string" | "bool" | "void" | "auto" | "array" | "dictionary" | "optional" | "i8" | "i16" | "i32" | "i64"
        | "u8" | "u16" | "u32" | "u64" | "f16" | "f32" | "f64" => base,
        _ => match base.split_once('.') {
            Some((alias, name)) => match ctx.links.iter().find(|l| l.alias == alias && !l.native) {
                Some(link) => format!("{}@{}", name, path_key(&link.target)),
                None => return UNKNOWN.into(),
            },
            None => match ctx.find(&base) {
                Some(s) if matches!(s.kind, SymKind::Def | SymKind::Enum) => format!("{}@{}", base, path_key(&ctx.path)),
                _ => return UNKNOWN.into(),
            },
        },
    };
    if base == "auto" {
        return UNKNOWN.into();
    }
    if args.is_empty() {
        match base.as_str() {
            "array" => "array<?>".into(),
            "dictionary" => "dictionary<?,?>".into(),
            _ => base,
        }
    } else {
        format!("{}<{}>", base, args.join(","))
    }
}

/// Can a value of type `actual` be stored where `target` is expected?
fn optional_inner(t: &str) -> Option<String> {
    let (base, args) = split_type(t);
    if base == "optional" {
        Some(args.first().cloned().unwrap_or_else(|| UNKNOWN.into()))
    } else {
        None
    }
}

pub fn assignable(target: &str, actual: &str) -> bool {
    if target == UNKNOWN || actual == UNKNOWN || target == actual {
        return true;
    }
    if let Some(inner) = optional_inner(target) {
        return actual == "null"
            || assignable(&inner, actual)
            || optional_inner(actual).map(|a| assignable(&inner, &a)).unwrap_or(false);
    }
    if optional_inner(actual).is_some() {
        return false; // may be null: check with `!= null` first
    }
    if is_numeric(target) && is_int(actual) {
        return true; // whole numbers widen and sized types convert
    }

    if is_float(target) && is_float(actual) {
        return true;
    }
    if actual == "null" {
        return !is_numeric(target) && target != "bool";
    }
    let (tb, ta) = split_type(target);
    let (ab, aa) = split_type(actual);
    if tb == ab && (ta.is_empty() || aa.is_empty()) {
        return true; // `Box { }` carries no type arguments: treat them as unknown
    }
    if tb != ab || ta.len() != aa.len() {
        return false;
    }
    ta.iter().zip(&aa).all(|(t, a)| t == UNKNOWN || a == UNKNOWN || t == a)
}

fn conversion_help(target: &str, actual: &str) -> Option<String> {
    if optional_inner(actual).is_some() {
        return Some("the value may be null: check it first, for example: if (value != null) { ... }".into());
    }
    if is_numeric(target) && actual == "string" {
        return Some("convert the text with Text.toNumber(...)".into());
    }
    if target == "string" && is_numeric(actual) {
        return Some("turn the number into text with .: \"\" or Text.format(\"{}\", value)".into());
    }
    if is_int(target) && is_float(actual) {
        return Some("use Math.floor, Math.round or Math.trunc, or declare a float".into());
    }
    if target == "bool" {
        return Some("compare explicitly, for example: value != 0".into());
    }
    None
}

/// Replaces whole-word type parameters (`T` in `array<T>`) by arguments.
fn substitute(t: &str, params: &[String], args: &[String]) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            match params.iter().position(|p| p == word) {
                Some(i) => out.push_str(&args[i]),
                None => out.push_str(word),
            }
            word.clear();
        }
    };
    for c in t.chars() {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

pub struct TypeChecker<'a> {
    a: &'a Analysis,
    index: &'a FileIndex,
    globals: HashMap<String, String>,
    scopes: Vec<HashMap<String, String>>,
    this_type: Option<String>,
    return_type: Option<String>,
    pub diags: Vec<Diag>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(a: &'a Analysis, index: &'a FileIndex) -> Self {
        TypeChecker { a, index, globals: HashMap::new(), scopes: Vec::new(), this_type: None, return_type: None, diags: Vec::new() }
    }

    fn column_of(&self, line: usize, name: &str) -> usize {
        self.a.tokens.iter().find(|t| t.line == line && t.text == name && t.kind != Tk::Eof).map(|t| t.column).unwrap_or(0)
    }

    fn error(&mut self, line: usize, anchor: &str, message: String, help: Option<String>) {
        let col = if anchor.is_empty() { 0 } else { self.column_of(line, anchor) };
        let len = if col == 0 { 0 } else { anchor.chars().count() };
        if self.diags.iter().any(|d| d.line == line && d.message == message) {
            return;
        }
        self.diags.push(Diag { line, col, len, severity: Severity::Error, message, help });
    }

    fn mismatch(&mut self, line: usize, anchor: &str, what: &str, target: &str, actual: &str) {
        if !assignable(target, actual) {
            let message = format!("{}: expected {}, got {}", what, show(target), show(actual));
            self.error(line, anchor, message, conversion_help(target, actual));
        }
    }

    // ------------------------------------------------------------ lookups

    /// Index of the file that declares a canonical `Name@path` type.
    fn index_for(&self, canonical: &str) -> Option<&'a FileIndex> {
        let path = canonical.split_once('@')?.1;
        let path = split_type(path).0;
        if path_key(&self.index.path) == path {
            return Some(self.index);
        }
        self.a.linked.values().find(|l| path_key(&l.path) == path)
    }

    fn def_sym(&self, canonical: &str) -> Option<(&'a Sym, &'a FileIndex)> {
        let name = canonical.split_once('@')?.0;
        let index = self.index_for(canonical)?;
        index.find(name).filter(|s| s.kind == SymKind::Def).map(|s| (s, index))
    }

    fn local(&self, name: &str) -> Option<String> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    fn declare(&mut self, name: &str, ty: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn linked_alias(&self, e: &Expr) -> Option<&'a FileIndex> {
        match &e.kind {
            ExprKind::Ident(n) if self.local(n).is_none() => self.a.linked.get(n.as_str()).filter(|l| !l.path.to_string_lossy().starts_with('@')),
            _ => None,
        }
    }

    fn native_alias(&self, e: &Expr) -> Option<&'static str> {
        let ExprKind::Ident(n) = &e.kind else { return None };
        if self.local(n).is_some() {
            return None;
        }
        let link = self.index.links.iter().find(|l| &l.alias == n && l.native)?;
        crate::stdlib::module_alias(&link.target.to_string_lossy()).map(|(k, _)| k)
    }

    // ------------------------------------------------------------ program

    pub fn check(&mut self, program: &Program) {
        self.scopes.push(HashMap::new());
        // Declared global types first: functions may use globals declared later.
        for item in &program.items {
            if let Item::Global { stmt, .. } = item {
                if let StmtKind::Var { name, ty: Some(t), .. } = &stmt.kind {
                    self.globals.insert(name.clone(), canon(&t.render(), self.index));
                }
            }
        }
        for item in &program.items {
            match item {
                Item::Global { stmt, .. } => {
                    if let StmtKind::Var { name, ty, init, .. } = &stmt.kind {
                        let declared = ty.as_ref().map(|t| canon(&t.render(), self.index));
                        if let Some(e) = init {
                            let actual = self.expr(e, declared.as_deref());
                            match &declared {
                                Some(d) => self.mismatch(stmt.line, name, &format!("'{}'", name), d, &actual),
                                None => {
                                    self.globals.insert(name.clone(), actual);
                                }
                            }
                        }
                    }
                }
                Item::Function { decl, .. } => self.function(decl, None),
                Item::Def { decl, .. } => {
                    let self_type = format!("{}@{}", decl.name, path_key(&self.index.path));
                    for f in &decl.fields {
                        if let Some(d) = &f.default {
                            let target = canon(&f.ty.render(), self.index);
                            let actual = self.expr(d, Some(&target));
                            self.mismatch(f.pos.line, &f.name, &format!("default of field '{}'", f.name), &target, &actual);
                        }
                    }
                    for m in &decl.methods {
                        self.function(m, Some(self_type.clone()));
                    }
                }
                Item::Stmt(s) => self.stmt(s),
                _ => {}
            }
        }
    }

    fn function(&mut self, f: &FnDecl, this_type: Option<String>) {
        let saved = (self.this_type.take(), self.return_type.take(), std::mem::take(&mut self.scopes));
        self.this_type = this_type;
        self.return_type = Some(canon(&f.ret.render(), self.index));
        let mut scope = HashMap::new();
        for (name, ty) in f.params.iter().zip(&f.param_types) {
            scope.insert(name.clone(), canon(&ty.render(), self.index));
        }
        self.scopes.push(scope);
        self.block(&f.body);
        self.this_type = saved.0;
        self.return_type = saved.1;
        self.scopes = saved.2;
    }

    fn block(&mut self, stmts: &[Stmt]) {
        self.scopes.push(HashMap::new());
        for s in stmts {
            self.stmt(s);
        }
        self.scopes.pop();
    }

    /// Variables that `x != null` (possibly joined by `and`) proves are not null.
    fn narrowed(&self, cond: &Expr) -> Vec<(String, String)> {
        match &cond.kind {
            ExprKind::Binary(op, a, b) if op == "and" => {
                let mut v = self.narrowed(a);
                v.extend(self.narrowed(b));
                v
            }
            ExprKind::Binary(op, a, b) if op == "!=" => {
                let name = match (&a.kind, &b.kind) {
                    (ExprKind::Ident(n), ExprKind::Null) | (ExprKind::Null, ExprKind::Ident(n)) => n,
                    _ => return Vec::new(),
                };
                let current = self.local(name).or_else(|| self.globals.get(name).cloned());
                match current.as_deref().and_then(optional_inner) {
                    Some(inner) => vec![(name.clone(), inner)],
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    fn condition(&mut self, e: &Expr, what: &str) {
        let t = self.expr(e, None);
        if t != UNKNOWN && t != "bool" {
            self.error(e.line, "", format!("{} needs a bool, got {}", what, show(&t)), conversion_help("bool", &t));
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Var { name, ty, init, .. } => {
                let declared = ty.as_ref().map(|t| canon(&t.render(), self.index));
                let actual = match init {
                    Some(e) => self.expr(e, declared.as_deref()),
                    None => UNKNOWN.into(),
                };
                if let (Some(d), Some(_)) = (&declared, init) {
                    self.mismatch(s.line, name, &format!("'{}'", name), d, &actual);
                }
                self.declare(name, declared.unwrap_or(actual));
            }
            StmtKind::Expr(e) => {
                self.expr(e, None);
            }
            StmtKind::Block(b) => self.block(b),
            StmtKind::If(c, then, otherwise) => {
                self.condition(c, "if");
                let narrowed = self.narrowed(c);
                self.scopes.push(narrowed.into_iter().collect());
                self.block(then);
                self.scopes.pop();
                if let Some(o) = otherwise {
                    self.block(o);
                }
            }
            StmtKind::While(c, b) => {
                self.condition(c, "while");
                let narrowed = self.narrowed(c);
                self.scopes.push(narrowed.into_iter().collect());
                self.block(b);
                self.scopes.pop();
            }
            StmtKind::DoWhile(b, c) => {
                self.block(b);
                self.condition(c, "do-while");
            }
            StmtKind::Match(subject, arms) => {
                let st = self.expr(subject, None);
                let mut covered: Vec<String> = Vec::new();
                let mut wildcard = false;
                for arm in arms {
                    for p in &arm.patterns {
                        match p {
                            None => wildcard = true,
                            Some(p) => {
                                let pt = self.expr(p, Some(&st));
                                if !assignable(&st, &pt) && !assignable(&pt, &st) {
                                    self.error(p.line, "", format!("match case is {}, but the value is {}", show(&pt), show(&st)), None);
                                }
                                if let ExprKind::Scope(_, variant) = &p.kind {
                                    covered.push(variant.clone());
                                }
                            }
                        }
                    }
                    self.block(&arm.body);
                }
                // Enums must be covered completely unless there is a `_` case.
                if !wildcard {
                    if let Some(index) = self.index_for(&st) {
                        let name = st.split_once('@').map(|(n, _)| n).unwrap_or("");
                        if let Some(e) = index.find(name).filter(|e| e.kind == SymKind::Enum) {
                            let missing: Vec<String> = e.children.iter().filter(|v| !covered.contains(&v.name)).map(|v| format!("{}::{}", name, v.name)).collect();
                            if !missing.is_empty() {
                                self.error(
                                    s.line,
                                    "match",
                                    format!("match does not cover {}", missing.join(", ")),
                                    Some(format!("add the missing cases, or a default case: _ => ...")),
                                );
                            }
                        }
                    }
                }
            }
            StmtKind::Guard(c, b) => {
                self.condition(c, "guard");
                self.block(b);
                // After `guard (x != null) else { ... }`, x is known not to be null.
                for (name, t) in self.narrowed(c) {
                    self.declare(&name, t);
                }
            }
            StmtKind::ForIn { name, ty, iterable, body, .. } => {
                let it = self.expr(iterable, None);
                let (base, args) = split_type(&it);
                let element = match base.as_str() {
                    "array" => args.first().cloned().unwrap_or_else(|| UNKNOWN.into()),
                    "dictionary" => args.first().cloned().unwrap_or_else(|| UNKNOWN.into()),
                    "string" => "string".into(),
                    "range" => "number".into(),
                    _ => UNKNOWN.into(),
                };
                if it != UNKNOWN && !matches!(base.as_str(), "array" | "dictionary" | "string" | "range") {
                    self.error(iterable.line, "", format!("cannot loop over {}", show(&it)), None);
                }
                let declared = ty.as_ref().map(|t| canon(&t.render(), self.index));
                if let Some(d) = &declared {
                    self.mismatch(s.line, name, &format!("loop variable '{}'", name), d, &element);
                }
                let mut scope = HashMap::new();
                scope.insert(name.clone(), declared.unwrap_or(element));
                self.scopes.push(scope);
                self.block(body);
                self.scopes.pop();
            }
            StmtKind::Try { body, binding, handler } => {
                self.block(body);
                let mut scope = HashMap::new();
                if let Some(b) = binding {
                    scope.insert(b.clone(), "string".to_string());
                }
                self.scopes.push(scope);
                self.block(handler);
                self.scopes.pop();
            }
            StmtKind::Return(value) => {
                let expected = self.return_type.clone();
                match (value, expected.as_deref()) {
                    (Some(e), Some("void")) => {
                        self.expr(e, None);
                        self.error(s.line, "return", "a void function cannot return a value".into(), Some("remove the value, or declare a return type".into()));
                    }
                    (Some(e), Some(t)) => {
                        let actual = self.expr(e, Some(t));
                        self.mismatch(s.line, "return", "return value", t, &actual);
                    }
                    (Some(e), None) => {
                        self.expr(e, None);
                    }
                    _ => {}
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    // ------------------------------------------------------------ expressions

    fn signature_return(signature: &str) -> String {
        let first = signature.split_whitespace().next().unwrap_or("");
        match first {
            "number" | "float" | "string" | "bool" | "void" | "array<string>" | "array<number>" => first.into(),
            _ => UNKNOWN.into(),
        }
    }

    fn signature_params(signature: &str) -> Vec<String> {
        let Some(inside) = signature.split_once('(').map(|(_, r)| r.trim_end_matches(')')) else { return Vec::new() };
        let mut out = Vec::new();
        let mut depth = 0;
        let mut current = String::new();
        for c in inside.chars() {
            match c {
                '<' => {
                    depth += 1;
                    current.push(c)
                }
                '>' => {
                    depth -= 1;
                    current.push(c)
                }
                ',' if depth == 0 => {
                    out.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        if !current.trim().is_empty() {
            out.push(current.trim().to_string());
        }
        out.iter()
            .map(|p| {
                let ty = p.rsplit_once(' ').map(|(t, _)| t).unwrap_or(p);
                match ty {
                    "number" | "float" | "string" | "bool" => ty.to_string(),
                    _ => UNKNOWN.into(),
                }
            })
            .collect()
    }

    fn check_args(&mut self, line: usize, callee: &str, params: &[String], args: &[Expr]) {
        for (i, arg) in args.iter().enumerate() {
            let expected = params.get(i).cloned().unwrap_or_else(|| UNKNOWN.into());
            let actual = self.expr(arg, Some(&expected));
            self.mismatch(line, callee, &format!("argument {} of {}", i + 1, callee), &expected, &actual);
        }
    }

    fn call_sym(&mut self, e: &Expr, sym: &Sym, ctx: &FileIndex, args: &[Expr]) -> String {
        let params: Vec<String> = sym.params.iter().map(|p| canon(p, ctx)).collect();
        self.check_args(e.line, &sym.name, &params, args);
        sym.ty.as_deref().map(|t| canon(t, ctx)).unwrap_or_else(|| UNKNOWN.into())
    }

    fn member_of(&mut self, base_type: &str, member: &str, line: usize) -> Option<(Sym, &'a FileIndex)> {
        if optional_inner(base_type).is_some() {
            self.error(
                line,
                member,
                format!("'{}' is read from a value that may be null ({})", member, show(base_type)),
                Some("check it first: if (value != null) { ... } or guard (value != null) else { return }".into()),
            );
            return None;
        }
        let (def, index) = self.def_sym(base_type)?;
        match def.children.iter().find(|c| c.name == member) {
            Some(m) => {
                let mut m = m.clone();
                // Box<number>.Value: replace the def's type parameters by the arguments.
                let args = split_type(base_type).1;
                if !def.params.is_empty() && args.len() == def.params.len() {
                    let subst = |t: &str| substitute(t, &def.params, &args);
                    m.ty = m.ty.as_deref().map(subst);
                    m.params = m.params.iter().map(|t| subst(t)).collect();
                }
                Some((m, index))
            }
            None => {
                let help = crate::diag::closest(member, def.children.iter().map(|c| c.name.as_str())).map(|c| format!("did you mean '{}'?", c));
                self.error(line, member, format!("{} has no member '{}'", def.name, member), help);
                None
            }
        }
    }

    fn expr(&mut self, e: &Expr, expected: Option<&str>) -> String {
        match &e.kind {
            ExprKind::Int(_) => "number".into(),
            ExprKind::Float(_) => "float".into(),
            ExprKind::Str(_) => "string".into(),
            ExprKind::Bool(_) => "bool".into(),
            ExprKind::Null => "null".into(),
            ExprKind::Ident(n) => self
                .local(n)
                .or_else(|| self.globals.get(n).cloned())
                .unwrap_or_else(|| UNKNOWN.into()),
            ExprKind::This => self.this_type.clone().unwrap_or_else(|| UNKNOWN.into()),
            ExprKind::Member(base, name) => {
                if let Some(linked) = self.linked_alias(base) {
                    return match linked.find(name) {
                        Some(s) if s.kind == SymKind::Variable => s.ty.as_deref().map(|t| canon(t, linked)).unwrap_or_else(|| UNKNOWN.into()),
                        _ => UNKNOWN.into(),
                    };
                }
                if let Some(module) = self.native_alias(base) {
                    return crate::stdlib::members(module)
                        .find(|(_, n, ..)| n == name)
                        .map(|(_, _, sig, _)| Self::signature_return(sig))
                        .unwrap_or_else(|| UNKNOWN.into());
                }
                let bt = self.expr(base, None);
                match self.member_of(&bt, name, e.line) {
                    Some((m, ctx)) if m.kind == SymKind::Field => m.ty.as_deref().map(|t| canon(t, ctx)).unwrap_or_else(|| UNKNOWN.into()),
                    _ => UNKNOWN.into(),
                }
            }

            ExprKind::Scope(base, _) => match &base.kind {
                ExprKind::Ident(n) => canon(n, self.index),
                ExprKind::Member(inner, n) => match &inner.kind {
                    ExprKind::Ident(alias) => canon(&format!("{}.{}", alias, n), self.index),
                    _ => UNKNOWN.into(),
                },
                _ => UNKNOWN.into(),
            },
            
            ExprKind::Index(base, key) => {
                let bt = self.expr(base, None);
                let kt = self.expr(key, None);
                let (b, args) = split_type(&bt);
                match b.as_str() {
                    "string" => "string".into(),
                    "array" if kt == "range" => bt.clone(),
                    "array" => {
                        if kt != UNKNOWN && !is_int(&kt) {
                            self.error(e.line, "", format!("array index must be a number, got {}", show(&kt)), None);
                        }
                        args.first().cloned().unwrap_or_else(|| UNKNOWN.into())
                    }
                    "dictionary" => {
                        if let Some(k) = args.first() {
                            self.mismatch(e.line, "", "dictionary key", k, &kt);
                        }
                        args.get(1).cloned().unwrap_or_else(|| UNKNOWN.into())
                    }
                    _ => UNKNOWN.into(),
                }
            }
            ExprKind::Call(callee, args) => self.call(e, callee, args),
            ExprKind::Unary(op, inner) => {
                let t = self.expr(inner, None);
                match op.as_str() {
                    "not" => {
                        if t != UNKNOWN && t != "bool" {
                            self.error(e.line, "", format!("'not' needs a bool, got {}", show(&t)), None);
                        }
                        "bool".into()
                    }
                    "-" | "+" => {
                        if t != UNKNOWN && !is_numeric(&t) {
                            self.error(e.line, "", format!("'{}' needs a number, got {}", op, show(&t)), None);
                        }
                        t
                    }
                    _ => "number".into(),
                }
            }
            ExprKind::Postfix(_, inner) => self.expr(inner, None),
            ExprKind::Binary(op, l, r) => {
                let lt = self.expr(l, None);
                let rt = self.expr(r, None);
                self.binary(e, op, &lt, &rt)
            }
            ExprKind::Assign(op, target, value) => {
                let tt = self.expr(target, None);
                if op == "=" {
                    let vt = self.expr(value, Some(&tt));
                    let anchor = match &target.kind {
                        ExprKind::Ident(n) | ExprKind::Member(_, n) => n.clone(),
                        _ => String::new(),
                    };
                    self.mismatch(e.line, &anchor, "assignment", &tt, &vt);
                    tt
                } else {
                    let vt = self.expr(value, None);
                    self.binary(e, &op[..op.len() - 1], &tt, &vt);
                    tt
                }
            }
            ExprKind::Ternary(c, a, b) => {
                self.condition(c, "'?'");
                let at = self.expr(a, expected);
                let bt = self.expr(b, expected);
                if at == bt {
                    at
                } else {
                    UNKNOWN.into()
                }
            }
            ExprKind::New(type_path, fields) => {
                let written = match &type_path.kind {
                    ExprKind::Ident(n) => n.clone(),
                    ExprKind::Member(inner, n) => match &inner.kind {
                        ExprKind::Ident(alias) => format!("{}.{}", alias, n),
                        _ => String::new(),
                    },
                    _ => String::new(),
                };
                let ty = canon(&written, self.index);
                let def = self.def_sym(&ty);
                for (name, value) in fields {
                    let field = def.and_then(|(d, ctx)| d.children.iter().find(|c| &c.name == name && c.kind == SymKind::Field).map(|f| (f.clone(), ctx)));
                    let target = field.and_then(|(f, ctx)| f.ty.as_deref().map(|t| canon(t, ctx))).unwrap_or_else(|| UNKNOWN.into());
                    let actual = self.expr(value, Some(&target));
                    self.mismatch(value.line, name, &format!("field '{}'", name), &target, &actual);
                }
                ty
            }
            ExprKind::ArrayLit(items) => {
                let element = expected
                    .map(split_type)
                    .filter(|(b, _)| b == "array")
                    .and_then(|(_, a)| a.first().cloned());
                let mut first: Option<String> = None;
                for item in items {
                    let t = self.expr(item, element.as_deref());
                    if let Some(el) = &element {
                        self.mismatch(item.line, "", "array element", el, &t);
                    }
                    if first.is_none() {
                        first = Some(t);
                    }
                }
                if let Some((b, _)) = expected.map(split_type) {
                    if b == "dictionary" && items.is_empty() {
                        return expected.unwrap().to_string();
                    }
                }
                match (element, first) {
                    (Some(el), _) => format!("array<{}>", el),
                    (None, Some(f)) if items.len() == 1 || f != UNKNOWN => format!("array<{}>", f),
                    _ => "array<?>".into(),
                }
            }
            ExprKind::DictLit(entries) => {
                for (k, v) in entries {
                    if let DictKey::Expr(k) = k {
                        self.expr(k, None);
                    }
                    self.expr(v, None);
                }
                expected.filter(|t| t.starts_with("dictionary")).map(String::from).unwrap_or_else(|| "dictionary<?,?>".into())
            }
            ExprKind::TypedCollection(ty, items) => {
                let t = canon(&ty.render(), self.index);
                let element = split_type(&t).1.first().cloned();
                for item in items {
                    let it = self.expr(item, element.as_deref());
                    if items.len() > 1 || !matches!(item.kind, ExprKind::ArrayLit(_) | ExprKind::DictLit(_)) {
                        if let Some(el) = &element {
                            if split_type(&t).0 == "array" {
                                self.mismatch(item.line, "", "array element", el, &it);
                            }
                        }
                    }
                }
                t
            }
            ExprKind::Lambda(decl) => {
                let saved = self.return_type.replace(canon(&decl.ret.render(), self.index));
                let mut scope = HashMap::new();
                for (name, ty) in decl.params.iter().zip(&decl.param_types) {
                    scope.insert(name.clone(), canon(&ty.render(), self.index));
                }
                self.scopes.push(scope);
                self.block(&decl.body);
                self.scopes.pop();
                self.return_type = saved;
                "function".into()
            }
            ExprKind::Typed(ty, inner) => {
                self.expr(inner, None);
                canon(&ty.render(), self.index)
            }
        }
    }

    fn binary(&mut self, e: &Expr, op: &str, lt: &str, rt: &str) -> String {
        let known = lt != UNKNOWN && rt != UNKNOWN;
        match op {
            ".:" => "string".into(),
            "==" | "!=" => "bool".into(),
            "and" | "or" => {
                for t in [lt, rt] {
                    if t != UNKNOWN && t != "bool" {
                        self.error(e.line, op, format!("'{}' needs bool values, got {}", op, show(t)), None);
                    }
                }
                "bool".into()
            }
            "<" | "<=" | ">" | ">=" => {
                let ok = (is_numeric(lt) && is_numeric(rt)) || (lt == "string" && rt == "string");
                if known && !ok {
                    self.error(e.line, op, format!("cannot compare {} with {}", show(lt), show(rt)), None);
                }
                "bool".into()
            }
            ".." => "range".into(),
            "+" | "-" | "*" | "/" | "%" => {
                if known && (lt == "string" || rt == "string") && op == "+" {
                    self.error(e.line, op, "'+' does not join text, use '.:'".into(), Some("join text with '.:', for example: \"total: \" .: score".into()));
                    return "string".into();
                }
                if known && !(is_numeric(lt) && is_numeric(rt)) {
                    self.error(e.line, op, format!("'{}' needs numbers, got {} and {}", op, show(lt), show(rt)), None);
                    return UNKNOWN.into();
                }
                if is_float(lt) || is_float(rt) {
                    "float".into()
                } else if is_int(lt) && is_int(rt) {
                    "number".into()
                } else {
                    UNKNOWN.into()
                }
            }
            "&" | "|" | "^" | "shl" | "shr" => {
                if known && !(is_int(lt) && is_int(rt)) {
                    self.error(e.line, op, format!("'{}' needs whole numbers, got {} and {}", op, show(lt), show(rt)), None);
                }
                "number".into()
            }
            _ => UNKNOWN.into(),
        }
    }

    fn call(&mut self, e: &Expr, callee: &Expr, args: &[Expr]) -> String {
        match &callee.kind {
            ExprKind::Ident(n) if self.local(n).is_none() => {
                if let Some(s) = self.index.find(n).filter(|s| s.kind == SymKind::Function) {
                    let s = s.clone();
                    let ctx = self.index;
                    return self.call_sym(e, &s, ctx, args);
                }
                let arg_types: Vec<String> = args.iter().map(|a| self.expr(a, None)).collect();
                match n.as_str() {
                    "len" | "find" => "number".into(),
                    "args" => "array<string>".into(),
                    "pop" | "remove" => split_type(arg_types.first().map(String::as_str).unwrap_or(UNKNOWN)).1.first().cloned().unwrap_or_else(|| UNKNOWN.into()),
                    "push" => {
                        if let Some(el) = split_type(arg_types.first().map(String::as_str).unwrap_or(UNKNOWN)).1.first() {
                            if split_type(&arg_types[0]).0 == "array" {
                                for t in &arg_types[1..] {
                                    self.mismatch(e.line, "push", "pushed value", el, t);
                                }
                            }
                        }
                        "void".into()
                    }
                    "post" | "warn" | "report" | "insert" | "sort" => "void".into(),
                    _ => UNKNOWN.into(),
                }
            }
            ExprKind::Member(base, name) => {
                if let Some(linked) = self.linked_alias(base) {
                    if let Some(s) = linked.find(name).filter(|s| s.kind == SymKind::Function) {
                        let s = s.clone();
                        return self.call_sym(e, &s, linked, args);
                    }
                    for a in args {
                        self.expr(a, None);
                    }
                    return UNKNOWN.into();
                }
                if let Some(module) = self.native_alias(base) {
                    if let Some((_, _, sig, _)) = crate::stdlib::members(module).find(|(_, n, ..)| n == name) {
                        let params = Self::signature_params(sig);
                        self.check_args(e.line, name, &params, args);
                        return Self::signature_return(sig);
                    }
                    return UNKNOWN.into();
                }
                let bt = self.expr(base, None);
                match self.member_of(&bt, name, e.line) {
                    Some((m, ctx)) if m.kind == SymKind::Method => self.call_sym(e, &m, ctx, args),
                    _ => {
                        for a in args {
                            self.expr(a, None);
                        }
                        UNKNOWN.into()
                    }
                }
            }
            _ => {
                self.expr(callee, None);
                for a in args {
                    self.expr(a, None);
                }
                UNKNOWN.into()
            }
        }
    }
}
