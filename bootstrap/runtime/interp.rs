//! Tree-walking interpreter for the stage-0 bootstrap.
//!
//! Semantics follow the CL++ reference: `def` objects, arrays and
//! dictionaries are reference values (assigning one shares it), strings are
//! immutable text indexed by Unicode character, `number` is a 64-bit integer
//! and `float` a 64-bit IEEE value. `report` raises an error that
//! `try`/`catch` can catch. Types are not checked here: that is the job of the
//! self-hosted checker.

use crate::ast::*;
use crate::stdlib::{self, SVal};
use crate::{lex, parse};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// ==================================================================== values

pub struct Str {
    text: String,
    ascii: bool,
    /// Byte offset of every character (plus the end), built on first index of
    /// a non-ASCII string so indexing stays O(1).
    offsets: std::cell::OnceCell<Vec<usize>>,
}

impl Str {
    fn new(text: String) -> Rc<Str> {
        let ascii = text.is_ascii();
        Rc::new(Str { text, ascii, offsets: std::cell::OnceCell::new() })
    }

    fn offsets(&self) -> &Vec<usize> {
        self.offsets.get_or_init(|| {
            let mut v: Vec<usize> = self.text.char_indices().map(|(i, _)| i).collect();
            v.push(self.text.len());
            v
        })
    }

    fn char_len(&self) -> usize {
        if self.ascii {
            self.text.len()
        } else {
            self.offsets().len() - 1
        }
    }

    fn slice(&self, start: usize, end: usize) -> String {
        if self.ascii {
            self.text[start..end].to_string()
        } else {
            let o = self.offsets();
            self.text[o[start]..o[end]].to_string()
        }
    }
}

pub struct DefInfo {
    decl: Rc<DefDecl>,
    module: Rc<Module>,
    field_index: HashMap<String, usize>,
    methods: HashMap<String, Rc<FnDecl>>,
}

pub struct EnumInfo {
    decl: Rc<EnumDecl>,
}

pub struct Object {
    def: Rc<DefInfo>,
    fields: RefCell<Vec<Value>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    Post,
    Warn,
    Report,
    Len,
    Push,
    Pop,
    Insert,
    Remove,
    Find,
    Sort,
    Args,
    /// `@clpp.<module>` member from stdlib.rs.
    Native(&'static str, &'static str),
}

impl Builtin {
    fn from_name(name: &str) -> Option<Builtin> {
        Some(match name {
            "post" => Builtin::Post,
            "warn" => Builtin::Warn,
            "report" => Builtin::Report,
            "len" => Builtin::Len,
            "push" => Builtin::Push,
            "pop" => Builtin::Pop,
            "insert" => Builtin::Insert,
            "remove" => Builtin::Remove,
            "find" => Builtin::Find,
            "sort" => Builtin::Sort,
            "args" => Builtin::Args,
            _ => return None,
        })
    }
}

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<Str>),
    Array(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<Vec<(Value, Value)>>>),
    Object(Rc<Object>),
    Enum(Rc<EnumInfo>, usize),
    EnumType(Rc<EnumInfo>),
    Type(Rc<DefInfo>),
    Func(Rc<FnDecl>, Rc<Module>),
    Builtin(Builtin),
    Module(Rc<Module>),
    Range(i64, i64),
}

impl Value {
    fn str(text: impl Into<String>) -> Value {
        Value::Str(Str::new(text.into()))
    }

    fn array(items: Vec<Value>) -> Value {
        Value::Array(Rc::new(RefCell::new(items)))
    }

    fn dict(entries: Vec<(Value, Value)>) -> Value {
        Value::Dict(Rc::new(RefCell::new(entries)))
    }

    fn type_name(&self) -> String {
        match self {
            Value::Null => "null".into(),
            Value::Bool(_) => "bool".into(),
            Value::Int(_) => "number".into(),
            Value::Float(_) => "float".into(),
            Value::Str(_) => "string".into(),
            Value::Array(_) => "array".into(),
            Value::Dict(_) => "dictionary".into(),
            Value::Object(o) => o.def.decl.name.clone(),
            Value::Enum(e, _) => e.decl.name.clone(),
            Value::EnumType(e) => format!("enum class {}", e.decl.name),
            Value::Type(d) => format!("def {}", d.decl.name),
            Value::Func(f, _) => format!("function {}", f.name),
            Value::Builtin(_) => "built-in function".into(),
            Value::Module(m) => format!("module {}", m.name),
            Value::Range(_, _) => "range".into(),
        }
    }
}

fn display(value: &Value, out: &mut String, depth: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => {
            let _ = write!(out, "{}", i);
        }
        Value::Float(f) => {
            if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e16 {
                let _ = write!(out, "{:.1}", f);
            } else {
                let _ = write!(out, "{}", f);
            }
        }
        Value::Str(s) => {
            if depth > 0 {
                let _ = write!(out, "{:?}", s.text);
            } else {
                out.push_str(&s.text);
            }
        }
        Value::Array(items) => {
            if depth > 4 {
                out.push_str("{...}");
                return;
            }
            out.push('{');
            for (i, item) in items.borrow().iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                display(item, out, depth + 1);
            }
            out.push('}');
        }
        Value::Dict(entries) => {
            if depth > 4 {
                out.push_str("{...}");
                return;
            }
            out.push('{');
            for (i, (k, v)) in entries.borrow().iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push('[');
                display(k, out, depth + 1);
                out.push_str("] = ");
                display(v, out, depth + 1);
            }
            out.push('}');
        }
        Value::Object(o) => {
            let _ = write!(out, "{} {{...}}", o.def.decl.name);
        }
        Value::Enum(e, i) => {
            let _ = write!(out, "{}::{}", e.decl.name, e.decl.variants[*i]);
        }
        Value::Range(a, b) => {
            let _ = write!(out, "{} .. {}", a, b);
        }
        other => out.push_str(&other.type_name()),
    }
}

pub fn to_text(value: &Value) -> String {
    let mut out = String::new();
    display(value, &mut out, 0);
    out
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => (*x as f64) == *y,
        (Value::Str(x), Value::Str(y)) => x.text == y.text,
        (Value::Enum(e1, i1), Value::Enum(e2, i2)) => Rc::ptr_eq(e1, e2) && i1 == i2,
        (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
        (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
        (Value::Dict(x), Value::Dict(y)) => Rc::ptr_eq(x, y),
        (Value::Type(x), Value::Type(y)) => Rc::ptr_eq(x, y),
        (Value::EnumType(x), Value::EnumType(y)) => Rc::ptr_eq(x, y),
        (Value::Module(x), Value::Module(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

// ==================================================================== control flow

pub struct RtError {
    pub message: String,
    pub location: Option<String>,
    pub trace: Vec<String>,
}

pub enum Flow {
    Break,
    Continue,
    Return(Value),
    Error(RtError),
}

type R<T> = Result<T, Flow>;

fn fail<T>(message: impl Into<String>) -> R<T> {
    Err(Flow::Error(RtError { message: message.into(), location: None, trace: Vec::new() }))
}

impl RtError {
    pub fn render(&self) -> String {
        let mut out = format!("error: {}", self.message);
        if let Some(loc) = &self.location {
            let _ = write!(out, "\n  at {}", loc);
        }
        for frame in &self.trace {
            let _ = write!(out, "\n  in {}", frame);
        }
        out
    }
}

// ==================================================================== modules

pub struct Module {
    pub name: String,
    pub path: PathBuf,
    /// Path for messages, without the Windows `\?\` verbatim prefix.
    pub shown: String,
    globals: RefCell<HashMap<String, Value>>,
    exports: RefCell<HashSet<String>>,
    /// Declared numeric types of globals (`u8 count`), enforced on assignment.
    types: RefCell<HashMap<String, String>>,
}

struct Frame {
    scopes: Vec<HashMap<String, Value>>,
    /// Declared numeric types of locals, innermost declaration wins.
    types: HashMap<String, String>,
    this: Option<Rc<Object>>,
    module: Rc<Module>,
}

impl Frame {
    fn new(module: Rc<Module>, this: Option<Rc<Object>>) -> Frame {
        Frame { scopes: vec![HashMap::new()], types: HashMap::new(), this, module }
    }
}

pub struct Interp {
    modules: HashMap<PathBuf, Rc<Module>>,
    loading: Vec<PathBuf>,
    pub args: Vec<String>,
    depth: usize,
}

const MAX_CALL_DEPTH: usize = 2000;

impl Interp {
    pub fn new(args: Vec<String>) -> Interp {
        Interp { modules: HashMap::new(), loading: Vec::new(), args, depth: 0 }
    }

    pub fn run_file(&mut self, path: &Path) -> Result<(), String> {
        match self.load_module(path) {
            Ok(_) => Ok(()),
            Err(Flow::Error(e)) => Err(render_error(&e)),
            Err(Flow::Return(_)) => Ok(()),
            Err(_) => Err("error: 'break' or 'continue' outside of a loop".into()),
        }
    }

    fn load_module(&mut self, path: &Path) -> R<Rc<Module>> {
        let canonical = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(e) => return fail(format!("cannot open '{}': {}", path.display(), e)),
        };
        if let Some(m) = self.modules.get(&canonical) {
            return Ok(m.clone());
        }
        if self.loading.contains(&canonical) {
            return fail(format!("circular link: '{}' links back to itself", path.display()));
        }
        let source = match std::fs::read_to_string(&canonical) {
            Ok(s) => s,
            Err(e) => return fail(format!("cannot read '{}': {}", path.display(), e)),
        };
        let display_name = path.display().to_string();
        let stem = canonical.file_stem().and_then(|s| s.to_str()).unwrap_or("Main").to_string();
        let tokens = lex::lex(&source, &display_name).map_err(|m| Flow::Error(RtError {
            message: m,
            location: None,
            trace: Vec::new(),
        }))?;
        let program = parse::parse(tokens, &display_name, &stem).map_err(|m| Flow::Error(RtError {
            message: m,
            location: None,
            trace: Vec::new(),
        }))?;

        let module = Rc::new(Module {
            name: program.module_name.clone(),
            path: canonical.clone(),
            shown: shown_path(&canonical),
            globals: RefCell::new(HashMap::new()),
            exports: RefCell::new(HashSet::new()),
            types: RefCell::new(HashMap::new()),
        });

        self.loading.push(canonical.clone());
        let result = self.init_module(&module, program, &display_name);
        self.loading.pop();
        result?;
        self.modules.insert(canonical, module.clone());
        Ok(module)
    }

    fn init_module(&mut self, module: &Rc<Module>, program: Program, file: &str) -> R<()> {
        let base_dir = module.path.parent().map(Path::to_path_buf).unwrap_or_default();

        // Pass 1: declarations become visible before any statement runs.
        for item in &program.items {
            let (name, value, exported) = match item {
                Item::Link { path, alias, .. } if path.starts_with('@') => {
                    let native = match native_module(path) {
                        Some(m) => m,
                        None => return fail(format!("{}: unknown standard module '{}'", file, path)),
                    };
                    let name = alias.clone().unwrap_or_else(|| native.name.clone());
                    (name, Value::Module(native), false)
                }
                Item::Link { path, alias, line, .. } => {
                    let target = base_dir.join(path);
                    let linked = self.load_module(&target).map_err(|flow| match flow {
                        Flow::Error(mut e) => {
                            if e.location.is_none() {
                                e.location = Some(format!("{}:{}", file, line));
                            } else {
                                e.trace.push(format!("link \"{}\" ({}:{})", path, file, line));
                            }
                            Flow::Error(e)
                        }
                        other => other,
                    })?;
                    let name = alias.clone().unwrap_or_else(|| linked.name.clone());
                    (name, Value::Module(linked), false)
                }
                Item::Function { decl, exported } => {
                    (decl.name.clone(), Value::Func(decl.clone(), module.clone()), *exported)
                }
                Item::Def { decl, exported } => {
                    let mut field_index = HashMap::new();
                    for (i, f) in decl.fields.iter().enumerate() {
                        field_index.insert(f.name.clone(), i);
                    }
                    let mut methods = HashMap::new();
                    for m in &decl.methods {
                        methods.insert(m.name.clone(), m.clone());
                    }
                    let info = Rc::new(DefInfo { decl: decl.clone(), module: module.clone(), field_index, methods });
                    (decl.name.clone(), Value::Type(info), *exported)
                }
                Item::Enum { decl, exported } => {
                    (decl.name.clone(), Value::EnumType(Rc::new(EnumInfo { decl: decl.clone() })), *exported)
                }
                Item::TypeAlias { name, exported, .. } => (name.clone(), Value::Null, *exported),
                Item::Global { .. } | Item::Stmt(_) => continue,
            };
            if module.globals.borrow().contains_key(&name) {
                return fail(format!("{}: '{}' is declared more than once", file, name));
            }
            module.globals.borrow_mut().insert(name.clone(), value);
            if exported {
                module.exports.borrow_mut().insert(name);
            }
        }

        // Pass 2: top-level statements run in order.
        let mut frame = Frame::new(module.clone(), None);
        for item in &program.items {
            match item {
                Item::Global { stmt, exported } => {
                    if let StmtKind::Var { name, ty, init, .. } = &stmt.kind {
                        let value = self
                            .init_value(&mut frame, ty.as_ref(), init.as_ref())
                            .map_err(|f| locate(f, file, stmt.line))?;
                        if module.globals.borrow().contains_key(name) {
                            return fail(format!("{}:{}: '{}' is declared more than once", file, stmt.line, name));
                        }
                        module.globals.borrow_mut().insert(name.clone(), value);
                        if let Some(t) = ty.as_ref().filter(|t| stdlib::is_numeric_type(&t.base)) {
                            module.types.borrow_mut().insert(name.clone(), t.base.clone());
                        }
                        if *exported {
                            module.exports.borrow_mut().insert(name.clone());
                        }
                    }
                }
                Item::Stmt(stmt) => match self.exec(&mut frame, stmt) {
                    Ok(()) => {}
                    Err(Flow::Return(_)) => return Ok(()),
                    Err(Flow::Error(mut e)) => {
                        if e.location.is_none() {
                            e.location = Some(format!("{}:{}", file, stmt.line));
                        }
                        return Err(Flow::Error(e));
                    }
                    Err(_) => return fail(format!("{}:{}: 'break' or 'continue' outside of a loop", file, stmt.line)),
                },
                _ => {}
            }
        }
        Ok(())
    }

    // ================================================================ statements

    fn exec_block(&mut self, frame: &mut Frame, stmts: &[Stmt]) -> R<()> {
        frame.scopes.push(HashMap::new());
        let mut result = Ok(());
        for stmt in stmts {
            if let Err(flow) = self.exec(frame, stmt) {
                result = Err(flow);
                break;
            }
        }
        frame.scopes.pop();
        result
    }

    fn exec(&mut self, frame: &mut Frame, stmt: &Stmt) -> R<()> {
        let file = frame.module.shown.clone();
        self.exec_inner(frame, stmt).map_err(|f| locate(f, &file, stmt.line))
    }

    fn exec_inner(&mut self, frame: &mut Frame, stmt: &Stmt) -> R<()> {
        match &stmt.kind {
            StmtKind::Var { name, ty, init, .. } => {
                let value = self.init_value(frame, ty.as_ref(), init.as_ref())?;
                let scope = frame.scopes.last_mut().unwrap();
                if scope.contains_key(name) {
                    return fail(format!("'{}' is already declared in this scope", name));
                }
                scope.insert(name.clone(), value);
                match ty.as_ref().filter(|t| stdlib::is_numeric_type(&t.base)) {
                    Some(t) => frame.types.insert(name.clone(), t.base.clone()),
                    None => frame.types.remove(name),
                };
                Ok(())
            }
            StmtKind::Expr(e) => {
                self.eval(frame, e)?;
                Ok(())
            }
            StmtKind::Block(body) => self.exec_block(frame, body),
            StmtKind::If(cond, then, otherwise) => {
                if self.condition(frame, cond)? {
                    self.exec_block(frame, then)
                } else if let Some(body) = otherwise {
                    self.exec_block(frame, body)
                } else {
                    Ok(())
                }
            }
            StmtKind::While(cond, body) => {
                while self.condition(frame, cond)? {
                    match self.exec_block(frame, body) {
                        Ok(()) | Err(Flow::Continue) => {}
                        Err(Flow::Break) => break,
                        Err(other) => return Err(other),
                    }
                }
                Ok(())
            }
            StmtKind::DoWhile(body, cond) => {
                loop {
                    match self.exec_block(frame, body) {
                        Ok(()) | Err(Flow::Continue) => {}
                        Err(Flow::Break) => break,
                        Err(other) => return Err(other),
                    }
                    if !self.condition(frame, cond)? {
                        break;
                    }
                }
                Ok(())
            }
            StmtKind::ForIn { name, iterable, body, .. } => {
                let items: Vec<Value> = match self.eval(frame, iterable)? {
                    Value::Array(a) => a.borrow().clone(),
                    Value::Dict(d) => d.borrow().iter().map(|(k, _)| k.clone()).collect(),
                    Value::Str(s) => s.text.chars().map(|c| Value::str(c.to_string())).collect(),
                    Value::Range(a, b) => (a..b).map(Value::Int).collect(),
                    other => return fail(format!("cannot iterate over {}", other.type_name())),
                };
                for item in items {
                    let mut scope = HashMap::new();
                    scope.insert(name.clone(), item);
                    frame.scopes.push(scope);
                    let result = self.exec_block(frame, body);
                    frame.scopes.pop();
                    match result {
                        Ok(()) | Err(Flow::Continue) => {}
                        Err(Flow::Break) => break,
                        Err(other) => return Err(other),
                    }
                }
                Ok(())
            }
            StmtKind::Guard(cond, body) => {
                if self.condition(frame, cond)? {
                    return Ok(());
                }
                self.exec_block(frame, body)?;
                fail("guard else block must leave the scope (return, break, continue or report)")
            }
            StmtKind::Try { body, binding, handler } => {
                let depth = frame.scopes.len();
                match self.exec_block(frame, body) {
                    Err(Flow::Error(e)) => {
                        frame.scopes.truncate(depth);
                        let mut scope = HashMap::new();
                        if let Some(name) = binding {
                            scope.insert(name.clone(), Value::str(e.message));
                        }
                        frame.scopes.push(scope);
                        let result = self.exec_block(frame, handler);
                        frame.scopes.pop();
                        result
                    }
                    other => other,
                }
            }
            StmtKind::Break => Err(Flow::Break),
            StmtKind::Continue => Err(Flow::Continue),
            StmtKind::Return(value) => {
                let v = match value {
                    Some(e) => self.eval(frame, e)?,
                    None => Value::Null,
                };
                Err(Flow::Return(v))
            }
        }
    }

    fn condition(&mut self, frame: &mut Frame, cond: &Expr) -> R<bool> {
        match self.eval(frame, cond)? {
            Value::Bool(b) => Ok(b),
            other => fail(format!("condition must be bool, got {}", other.type_name())),
        }
    }

    fn init_value(&mut self, frame: &mut Frame, ty: Option<&TypeName>, init: Option<&Expr>) -> R<Value> {
        match init {
            Some(e) => {
                let v = self.eval_typed(frame, e, ty)?;
                match ty.filter(|t| stdlib::is_numeric_type(&t.base)) {
                    Some(t) => coerce_number(&t.base, v, is_literal(e)),
                    None => Ok(v),
                }
            }
            None => Ok(match ty {
                Some(t) => default_for(t),
                None => Value::Null,
            }),
        }
    }

    /// Target typing: an empty `{}` becomes a dictionary when the declared
    /// type is `dictionary<...>`.
    fn eval_typed(&mut self, frame: &mut Frame, e: &Expr, ty: Option<&TypeName>) -> R<Value> {
        if let (ExprKind::ArrayLit(items), Some(t)) = (&e.kind, ty) {
            if t.base == "dictionary" {
                if items.is_empty() {
                    return Ok(Value::dict(Vec::new()));
                }
                return fail("an array literal cannot initialize a dictionary");
            }
        }
        if let (ExprKind::DictLit(_), Some(t)) = (&e.kind, ty) {
            if t.base == "array" {
                return fail("a dictionary literal cannot initialize an array");
            }
        }
        self.eval(frame, e)
    }

    // ================================================================ expressions

    fn lookup(&self, frame: &Frame, name: &str) -> R<Value> {
        for scope in frame.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Ok(v.clone());
            }
        }
        if let Some(v) = frame.module.globals.borrow().get(name) {
            return Ok(v.clone());
        }
        if let Some(b) = Builtin::from_name(name) {
            return Ok(Value::Builtin(b));
        }
        fail(format!("'{}' is not defined", name))
    }

    fn assign_name(&self, frame: &mut Frame, name: &str, value: Value) -> R<()> {
        let in_scope = frame.scopes.iter().any(|s| s.contains_key(name));
        let declared = if in_scope { frame.types.get(name).cloned() } else { frame.module.types.borrow().get(name).cloned() };
        let value = match declared {
            Some(t) => coerce_number(&t, value, false)?,
            None => value,
        };
        for scope in frame.scopes.iter_mut().rev() {
            if let Some(slot) = scope.get_mut(name) {
                *slot = value;
                return Ok(());
            }
        }
        let mut globals = frame.module.globals.borrow_mut();
        if let Some(slot) = globals.get_mut(name) {
            if matches!(slot, Value::Func(..) | Value::Type(_) | Value::EnumType(_) | Value::Module(_)) {
                return fail(format!("cannot assign to '{}'", name));
            }
            *slot = value;
            return Ok(());
        }
        fail(format!("'{}' is not defined", name))
    }

    fn eval(&mut self, frame: &mut Frame, e: &Expr) -> R<Value> {
        match &e.kind {
            ExprKind::Int(i) => Ok(Value::Int(*i)),
            ExprKind::Float(f) => Ok(Value::Float(*f)),
            ExprKind::Str(s) => Ok(Value::str(s.to_string())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Null => Ok(Value::Null),
            ExprKind::Ident(name) => self.lookup(frame, name),
            ExprKind::This => match &frame.this {
                Some(o) => Ok(Value::Object(o.clone())),
                None => fail("'@this' used outside of a method"),
            },
            ExprKind::Member(base, name) => {
                let target = self.eval(frame, base)?;
                self.get_member(&target, name)
            }
            ExprKind::Scope(base, name) => match self.eval(frame, base)? {
                Value::EnumType(info) => match info.decl.variants.iter().position(|v| v == name) {
                    Some(i) => Ok(Value::Enum(info.clone(), i)),
                    None => fail(format!("enum class {} has no member '{}'", info.decl.name, name)),
                },
                other => fail(format!("'::' needs an enum class, got {}", other.type_name())),
            },
            ExprKind::Index(base, index) => {
                let target = self.eval(frame, base)?;
                let key = self.eval(frame, index)?;
                get_index(&target, &key)
            }
            ExprKind::Call(callee, args) => self.call_expr(frame, callee, args),
            ExprKind::Unary(op, operand) => {
                let v = self.eval(frame, operand)?;
                match (op.as_str(), v) {
                    ("not", Value::Bool(b)) => Ok(Value::Bool(!b)),
                    ("-", Value::Int(i)) => Ok(Value::Int(i.wrapping_neg())),
                    ("-", Value::Float(f)) => Ok(Value::Float(-f)),
                    ("+", v @ (Value::Int(_) | Value::Float(_))) => Ok(v),
                    ("~", Value::Int(i)) => Ok(Value::Int(!i)),
                    (op, v) => fail(format!("operator '{}' cannot be applied to {}", op, v.type_name())),
                }
            }
            ExprKind::Postfix(op, target) => {
                let delta = if op == "++" { 1 } else { -1 };
                let mut old = Value::Null;
                self.update(frame, target, &mut |current| {
                    old = current.clone();
                    match current {
                        Value::Int(i) => Ok(Value::Int(i.wrapping_add(delta))),
                        Value::Float(f) => Ok(Value::Float(f + delta as f64)),
                        other => fail(format!("'{}' needs a number, got {}", op, other.type_name())),
                    }
                })?;
                Ok(old)
            }
            ExprKind::Binary(op, left, right) => {
                if op == "and" || op == "or" {
                    let l = match self.eval(frame, left)? {
                        Value::Bool(b) => b,
                        other => return fail(format!("'{}' needs bool operands, got {}", op, other.type_name())),
                    };
                    if (op == "and" && !l) || (op == "or" && l) {
                        return Ok(Value::Bool(l));
                    }
                    return match self.eval(frame, right)? {
                        Value::Bool(b) => Ok(Value::Bool(b)),
                        other => fail(format!("'{}' needs bool operands, got {}", op, other.type_name())),
                    };
                }
                let l = self.eval(frame, left)?;
                let r = self.eval(frame, right)?;
                binary(op, &l, &r)
            }
            ExprKind::Assign(op, target, value) => {
                if op == "=" {
                    let v = self.eval(frame, value)?;
                    self.store(frame, target, v.clone())?;
                    return Ok(v);
                }
                let rhs = self.eval(frame, value)?;
                let bin = &op[..op.len() - 1];
                let mut result = Value::Null;
                self.update(frame, target, &mut |current| {
                    let v = binary(bin, &current, &rhs)?;
                    result = v.clone();
                    Ok(v)
                })?;
                Ok(result)
            }
            ExprKind::Ternary(cond, yes, no) => {
                if self.condition(frame, cond)? {
                    self.eval(frame, yes)
                } else {
                    self.eval(frame, no)
                }
            }
            ExprKind::New(type_path, fields) => {
                let info = match self.eval(frame, type_path)? {
                    Value::Type(info) => info,
                    other => return fail(format!("'{{ }}' construction needs a def type, got {}", other.type_name())),
                };
                let mut provided: Vec<Option<Value>> = vec![None; info.decl.fields.len()];
                for (name, expr) in fields {
                    let index = match info.field_index.get(name) {
                        Some(i) => *i,
                        None => return fail(format!("def {} has no field '{}'", info.decl.name, name)),
                    };
                    let ty = info.decl.fields[index].ty.clone();
                    let mut v = self.eval_typed(frame, expr, Some(&ty))?;
                    if stdlib::is_numeric_type(&ty.base) {
                        v = coerce_number(&ty.base, v, is_literal(expr))?;
                    }
                    provided[index] = Some(v);
                }
                let mut values = Vec::with_capacity(provided.len());
                for (i, slot) in provided.into_iter().enumerate() {
                    let v = match slot {
                        Some(v) => v,
                        None => {
                            let field = &info.decl.fields[i];
                            match &field.default {
                                Some(expr) => {
                                    let mut def_frame = Frame::new(info.module.clone(), None);
                                    let v = self.eval_typed(&mut def_frame, expr, Some(&field.ty))?;
                                    if stdlib::is_numeric_type(&field.ty.base) {
                                        coerce_number(&field.ty.base, v, is_literal(expr))?
                                    } else {
                                        v
                                    }
                                }
                                None => default_for(&field.ty),
                            }
                        }
                    };
                    values.push(v);
                }
                Ok(Value::Object(Rc::new(Object { def: info, fields: RefCell::new(values) })))
            }
            ExprKind::ArrayLit(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.eval(frame, item)?);
                }
                Ok(Value::array(values))
            }
            ExprKind::DictLit(entries) => {
                let mut values: Vec<(Value, Value)> = Vec::with_capacity(entries.len());
                for (key, expr) in entries {
                    let k = match key {
                        DictKey::Name(n) => Value::str(n.clone()),
                        DictKey::Expr(k) => self.eval(frame, k)?,
                    };
                    let v = self.eval(frame, expr)?;
                    if values.iter().any(|(existing, _)| values_equal(existing, &k)) {
                        return fail(format!("duplicate dictionary key {}", to_text(&k)));
                    }
                    values.push((k, v));
                }
                Ok(Value::dict(values))
            }
            ExprKind::TypedCollection(ty, args) => self.typed_collection(frame, ty, args),
            ExprKind::Typed(ty, inner) => {
                let v = self.eval(frame, inner)?;
                self.check_type(frame, ty, v, is_literal(inner))
            }
        }
    }

    /// `(Type value)`: the value must have exactly this type; numbers are
    /// converted to sized types.
    fn check_type(&mut self, frame: &mut Frame, ty: &TypeName, v: Value, exact: bool) -> R<Value> {
        let name = ty.base.as_str();
        if stdlib::is_numeric_type(name) {
            return coerce_number(name, v, exact);
        }
        let ok = match name {
            "string" => matches!(v, Value::Str(_)),
            "bool" => matches!(v, Value::Bool(_)),
            "array" => matches!(v, Value::Array(_)),
            "dictionary" => matches!(v, Value::Dict(_)),
            _ => {
                let declared = match name.split_once('.') {
                    Some((alias, member)) => match self.lookup(frame, alias)? {
                        Value::Module(m) => self.get_member(&Value::Module(m), member)?,
                        other => return fail(format!("'{}' is not a module, got {}", alias, other.type_name())),
                    },
                    None => self.lookup(frame, name)?,
                };
                match (&declared, &v) {
                    (Value::Type(d), Value::Object(o)) => Rc::ptr_eq(d, &o.def),
                    (Value::EnumType(e), Value::Enum(ev, _)) => Rc::ptr_eq(e, ev),
                    (Value::Type(_), _) | (Value::EnumType(_), _) => false,
                    (Value::Null, _) => true, // `def Name = ...` structural types are not checked yet
                    (other, _) => return fail(format!("'{}' is not a type, it is a {}", name, other.type_name())),
                }
            }
        };
        if ok {
            Ok(v)
        } else {
            fail(format!("expected {}, got {} ({})", ty.render(), v.type_name(), to_text(&v)))
        }
    }

    fn typed_collection(&mut self, frame: &mut Frame, ty: &TypeName, args: &[Expr]) -> R<Value> {
        let single_literal = args.len() == 1 && matches!(args[0].kind, ExprKind::ArrayLit(_) | ExprKind::DictLit(_));
        match ty.base.as_str() {
            "array" => {
                if single_literal {
                    return self.eval_typed(frame, &args[0], Some(ty));
                }
                let mut values = Vec::with_capacity(args.len());
                for a in args {
                    values.push(self.eval(frame, a)?);
                }
                Ok(Value::array(values))
            }
            "dictionary" => {
                if args.is_empty() {
                    return Ok(Value::dict(Vec::new()));
                }
                if args.len() == 1 {
                    return match self.eval_typed(frame, &args[0], Some(ty))? {
                        Value::Dict(d) => Ok(Value::dict(d.borrow().clone())),
                        other => fail(format!("dictionary<...>() needs a dictionary, got {}", other.type_name())),
                    };
                }
                fail("dictionary<...>() takes one dictionary literal")
            }
            other => fail(format!("'{}<...>(...)' is not a collection constructor", other)),
        }
    }

    fn get_member(&self, target: &Value, name: &str) -> R<Value> {
        match target {
            Value::Object(o) => match o.def.field_index.get(name) {
                Some(i) => Ok(o.fields.borrow()[*i].clone()),
                None if o.def.methods.contains_key(name) => {
                    fail(format!("method '{}' must be called: {}.{}(...)", name, o.def.decl.name, name))
                }
                None => fail(format!("{} has no field '{}'", o.def.decl.name, name)),
            },
            Value::Module(m) => {
                let globals = m.globals.borrow();
                match globals.get(name) {
                    Some(v) if m.exports.borrow().contains(name) => Ok(v.clone()),
                    Some(_) => fail(format!("'{}' is not exported by module {}", name, m.name)),
                    None => fail(format!("module {} has no member '{}'", m.name, name)),
                }
            }
            other => fail(format!("{} has no member '{}'", other.type_name(), name)),
        }
    }

    /// Writes `value` to an assignable expression.
    fn store(&mut self, frame: &mut Frame, target: &Expr, value: Value) -> R<()> {
        match &target.kind {
            ExprKind::Ident(name) => self.assign_name(frame, name, value),
            ExprKind::Member(base, name) => {
                let obj = self.eval(frame, base)?;
                set_member(&obj, name, value)
            }
            ExprKind::Index(base, index) => {
                let container = self.eval(frame, base)?;
                let key = self.eval(frame, index)?;
                set_index(&container, key, value)
            }
            _ => fail("left side of '=' is not assignable"),
        }
    }

    /// Read-modify-write for `+=`, `++` and friends. The container and key are
    /// evaluated once.
    fn update(&mut self, frame: &mut Frame, target: &Expr, f: &mut dyn FnMut(Value) -> R<Value>) -> R<()> {
        match &target.kind {
            ExprKind::Ident(name) => {
                let current = self.lookup(frame, name)?;
                let next = f(current)?;
                self.assign_name(frame, name, next)
            }
            ExprKind::Member(base, name) => {
                let obj = self.eval(frame, base)?;
                let current = self.get_member(&obj, name)?;
                let next = f(current)?;
                set_member(&obj, name, next)
            }
            ExprKind::Index(base, index) => {
                let container = self.eval(frame, base)?;
                let key = self.eval(frame, index)?;
                let current = get_index(&container, &key)?;
                let next = f(current)?;
                set_index(&container, key, next)
            }
            _ => fail("left side of the assignment is not assignable"),
        }
    }

    // ================================================================ calls

    fn call_expr(&mut self, frame: &mut Frame, callee: &Expr, arg_exprs: &[Expr]) -> R<Value> {
        // Method call: obj.Method(...)
        if let ExprKind::Member(base, name) = &callee.kind {
            let target = self.eval(frame, base)?;
            if let Value::Object(obj) = &target {
                if let Some(method) = obj.def.methods.get(name).cloned() {
                    let args = self.eval_args(frame, arg_exprs)?;
                    let module = obj.def.module.clone();
                    let label = format!("{}.{}", obj.def.decl.name, method.name);
                    return self.call_function(&method, module, Some(obj.clone()), args, &label);
                }
            }
            let f = self.get_member(&target, name)?;
            let args = self.eval_args(frame, arg_exprs)?;
            return self.call_value(f, args);
        }
        let f = self.eval(frame, callee)?;
        let args = self.eval_args(frame, arg_exprs)?;
        self.call_value(f, args)
    }

    fn eval_args(&mut self, frame: &mut Frame, arg_exprs: &[Expr]) -> R<Vec<Value>> {
        let mut args = Vec::with_capacity(arg_exprs.len());
        for a in arg_exprs {
            args.push(self.eval(frame, a)?);
        }
        Ok(args)
    }

    fn call_value(&mut self, f: Value, args: Vec<Value>) -> R<Value> {
        match f {
            Value::Func(decl, module) => {
                let label = format!("{}.{}", module.name, decl.name);
                self.call_function(&decl, module, None, args, &label)
            }
            Value::Builtin(b) => self.call_builtin(b, args),
            Value::Type(info) => fail(format!("use '{} {{ Field = value }}' to construct a def", info.decl.name)),
            other => fail(format!("{} is not callable", other.type_name())),
        }
    }

    fn call_function(
        &mut self,
        decl: &Rc<FnDecl>,
        module: Rc<Module>,
        this: Option<Rc<Object>>,
        args: Vec<Value>,
        label: &str,
    ) -> R<Value> {
        if args.len() != decl.params.len() {
            return fail(format!(
                "{} expects {} argument(s), got {}",
                label,
                decl.params.len(),
                args.len()
            ));
        }
        if self.depth >= MAX_CALL_DEPTH {
            return fail(format!("call depth limit ({}) exceeded in {}", MAX_CALL_DEPTH, label));
        }
        let mut frame = Frame::new(module, this);
        for (name, value) in decl.params.iter().zip(args) {
            frame.scopes[0].insert(name.clone(), value);
        }
        self.depth += 1;
        let mut result = Ok(Value::Null);
        for stmt in &decl.body {
            if let Err(flow) = self.exec(&mut frame, stmt) {
                result = match flow {
                    Flow::Return(v) => Ok(v),
                    Flow::Error(mut e) => {
                        e.trace.push(format!("{} ({}:{})", label, frame.module.shown, decl.line));
                        Err(Flow::Error(e))
                    }
                    _ => fail(format!("'break' or 'continue' outside of a loop in {}", label)),
                };
                break;
            }
        }
        self.depth -= 1;
        result
    }

    fn call_builtin(&mut self, b: Builtin, args: Vec<Value>) -> R<Value> {
        let arity = |n: usize, name: &str| -> R<()> {
            if args.len() != n {
                return fail(format!("{} expects {} argument(s), got {}", name, n, args.len()));
            }
            Ok(())
        };
        match b {
            Builtin::Post => {
                arity(1, "post")?;
                println!("{}", to_text(&args[0]));
                Ok(Value::Null)
            }
            Builtin::Warn => {
                arity(1, "warn")?;
                eprintln!("warning: {}", to_text(&args[0]));
                Ok(Value::Null)
            }
            Builtin::Report => {
                arity(1, "report")?;
                fail(to_text(&args[0]))
            }
            Builtin::Len => {
                arity(1, "len")?;
                Ok(Value::Int(match &args[0] {
                    Value::Str(s) => s.char_len() as i64,
                    Value::Array(a) => a.borrow().len() as i64,
                    Value::Dict(d) => d.borrow().len() as i64,
                    other => return fail(format!("len() needs a string or collection, got {}", other.type_name())),
                }))
            }
            Builtin::Push => {
                if args.len() < 2 {
                    return fail("push expects an array and at least one value");
                }
                match &args[0] {
                    Value::Array(a) => {
                        a.borrow_mut().extend(args[1..].iter().cloned());
                        Ok(Value::Null)
                    }
                    other => fail(format!("push() needs an array, got {}", other.type_name())),
                }
            }
            Builtin::Pop => {
                arity(1, "pop")?;
                match &args[0] {
                    Value::Array(a) => match a.borrow_mut().pop() {
                        Some(v) => Ok(v),
                        None => fail("pop() on an empty array"),
                    },
                    other => fail(format!("pop() needs an array, got {}", other.type_name())),
                }
            }
            Builtin::Insert => {
                arity(3, "insert")?;
                match (&args[0], &args[1]) {
                    (Value::Array(a), Value::Int(i)) => {
                        let mut items = a.borrow_mut();
                        if *i < 0 || *i as usize > items.len() {
                            return fail(format!("insert index {} is out of range (length {})", i, items.len()));
                        }
                        items.insert(*i as usize, args[2].clone());
                        Ok(Value::Null)
                    }
                    _ => fail("insert() expects (array, number, value)"),
                }
            }
            Builtin::Remove => {
                arity(2, "remove")?;
                match (&args[0], &args[1]) {
                    (Value::Array(a), Value::Int(i)) => {
                        let mut items = a.borrow_mut();
                        if *i < 0 || *i as usize >= items.len() {
                            return fail(format!("remove index {} is out of range (length {})", i, items.len()));
                        }
                        Ok(items.remove(*i as usize))
                    }
                    (Value::Dict(d), key) => {
                        let mut entries = d.borrow_mut();
                        match entries.iter().position(|(k, _)| values_equal(k, key)) {
                            Some(p) => Ok(entries.remove(p).1),
                            None => fail(format!("key {} not found", to_text(key))),
                        }
                    }
                    _ => fail("remove() expects (array, number) or (dictionary, key)"),
                }
            }
            Builtin::Find => {
                arity(2, "find")?;
                match (&args[0], &args[1]) {
                    (Value::Array(a), needle) => Ok(Value::Int(
                        a.borrow().iter().position(|v| values_equal(v, needle)).map(|p| p as i64).unwrap_or(-1),
                    )),
                    (Value::Str(s), Value::Str(needle)) => Ok(Value::Int(match s.text.find(&needle.text) {
                        Some(byte) => s.text[..byte].chars().count() as i64,
                        None => -1,
                    })),
                    (Value::Dict(d), key) => Ok(Value::Bool(d.borrow().iter().any(|(k, _)| values_equal(k, key)))),
                    _ => fail("find() expects (array, value), (string, string) or (dictionary, key)"),
                }
            }
            Builtin::Sort => {
                arity(1, "sort")?;
                match &args[0] {
                    Value::Array(a) => {
                        let mut items = a.borrow_mut();
                        let mut error = None;
                        items.sort_by(|x, y| match compare(x, y) {
                            Ok(o) => o,
                            Err(_) => {
                                error = Some(());
                                std::cmp::Ordering::Equal
                            }
                        });
                        if error.is_some() {
                            return fail("sort() needs an array of numbers or of strings");
                        }
                        Ok(Value::Null)
                    }
                    other => fail(format!("sort() needs an array, got {}", other.type_name())),
                }
            }
            Builtin::Args => {
                arity(0, "args")?;
                Ok(Value::array(self.args.iter().map(|a| Value::str(a.clone())).collect()))
            }
            Builtin::Native(module, member) => {
                let mut others = Vec::new();
                let converted: Vec<SVal> = args.iter().map(|v| to_sval(v, &mut others)).collect();
                match stdlib::call(module, member, &converted) {
                    Ok(result) => Ok(from_sval(result, &others)),
                    Err(message) => fail(message),
                }
            }
        }
    }
}

// ==================================================================== helpers

/// `link @clpp.math as Math` and the other standard modules.
fn native_module(name: &str) -> Option<Rc<Module>> {
    let (key, short) = stdlib::module_alias(name)?;
    let module = Module {
        name: short.into(),
        path: PathBuf::from(name),
        shown: name.into(),
        globals: RefCell::new(HashMap::new()),
        exports: RefCell::new(HashSet::new()),
        types: RefCell::new(HashMap::new()),
    };
    for (m, member, signature, _) in stdlib::members(key) {
        let value = if stdlib::is_constant(signature) {
            match stdlib::call(m, member, &[]) {
                Ok(v) => from_sval(v, &[]),
                Err(_) => Value::Null,
            }
        } else {
            Value::Builtin(Builtin::Native(m, member))
        };
        module.globals.borrow_mut().insert(member.to_string(), value);
        module.exports.borrow_mut().insert(member.to_string());
    }
    Some(Rc::new(module))
}

/// Interpreter value -> stdlib value. Objects and enums are kept in `others`.
fn to_sval(v: &Value, others: &mut Vec<Value>) -> SVal {
    match v {
        Value::Null => SVal::Null,
        Value::Bool(b) => SVal::Bool(*b),
        Value::Int(i) => SVal::Int(*i),
        Value::Float(f) => SVal::Float(*f),
        Value::Str(s) => SVal::Str(s.text.clone()),
        Value::Array(items) => SVal::List(items.borrow().iter().map(|x| to_sval(x, others)).collect()),
        Value::Dict(entries) => SVal::Map(entries.borrow().iter().map(|(k, x)| (to_sval(k, others), to_sval(x, others))).collect()),
        other => {
            let id = match others.iter().position(|o| values_equal(o, other)) {
                Some(i) => i,
                None => {
                    others.push(other.clone());
                    others.len() - 1
                }
            };
            SVal::Other(id, to_text(other))
        }
    }
}

fn from_sval(v: SVal, others: &[Value]) -> Value {
    match v {
        SVal::Null => Value::Null,
        SVal::Bool(b) => Value::Bool(b),
        SVal::Int(i) => Value::Int(i),
        SVal::Float(f) => Value::Float(f),
        SVal::Str(s) => Value::str(s),
        SVal::List(items) => Value::array(items.into_iter().map(|x| from_sval(x, others)).collect()),
        SVal::Map(entries) => Value::dict(entries.into_iter().map(|(k, x)| (from_sval(k, others), from_sval(x, others))).collect()),
        SVal::Other(id, _) => others.get(id).cloned().unwrap_or(Value::Null),
    }
}

fn is_literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) => true,
        ExprKind::Unary(op, inner) => op == "-" && matches!(inner.kind, ExprKind::Int(_) | ExprKind::Float(_)),
        _ => false,
    }
}

/// Converts a value to a sized numeric type (see stdlib::coerce).
fn coerce_number(type_name: &str, v: Value, exact: bool) -> R<Value> {
    let s = match &v {
        Value::Int(i) => SVal::Int(*i),
        Value::Float(f) => SVal::Float(*f),
        other => return fail(format!("{} needs a number, got {}", type_name, other.type_name())),
    };
    match stdlib::coerce(type_name, &s, exact) {
        Ok(SVal::Int(i)) => Ok(Value::Int(i)),
        Ok(SVal::Float(f)) => Ok(Value::Float(f)),
        Ok(_) => Ok(v),
        Err(m) => fail(m),
    }
}

/// A runtime, lexer or parser error with the source line and a suggestion.
fn render_error(e: &RtError) -> String {
    use crate::diag;
    let read = |file: &str| std::fs::read_to_string(file).ok();
    if let Some(location) = &e.location {
        if let Some((file, line)) = location.rsplit_once(':') {
            if let Ok(line) = line.parse::<usize>() {
                // Lexer/parser messages carry their own column.
                if let Some(mut located) = diag::from_located(&e.message) {
                    located.trace = e.trace.clone();
                    let source = read(&located.file);
                    return diag::render(&located, source.as_deref());
                }
                let report = diag::Report {
                    severity: "error",
                    message: e.message.clone(),
                    file: file.to_string(),
                    line,
                    col: 0,
                    len: 0,
                    help: None,
                    trace: e.trace.clone(),
                };
                return diag::render(&report, read(file).as_deref());
            }
        }
    }
    if let Some(mut located) = diag::from_located(&e.message) {
        located.trace = e.trace.clone();
        let source = read(&located.file);
        return diag::render(&located, source.as_deref());
    }
    e.render()
}

fn shown_path(path: &Path) -> String {
    let text = path.display().to_string();
    text.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(text)
}

fn locate(flow: Flow, file: &str, line: usize) -> Flow {
    match flow {
        Flow::Error(mut e) => {
            if e.location.is_none() {
                e.location = Some(format!("{}:{}", file, line));
            }
            Flow::Error(e)
        }
        other => other,
    }
}

fn default_for(ty: &TypeName) -> Value {
    match ty.base.as_str() {
        "number" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => Value::Int(0),
        "float" | "f32" | "f64" => Value::Float(0.0),
        "string" => Value::str(""),
        "bool" => Value::Bool(false),
        "array" => Value::array(Vec::new()),
        "dictionary" => Value::dict(Vec::new()),
        _ => Value::Null,
    }
}

fn set_member(obj: &Value, name: &str, value: Value) -> R<()> {
    match obj {
        Value::Object(o) => match o.def.field_index.get(name) {
            Some(i) => {
                let ty = &o.def.decl.fields[*i].ty.base;
                let value = if stdlib::is_numeric_type(ty) { coerce_number(ty, value, false)? } else { value };
                o.fields.borrow_mut()[*i] = value;
                Ok(())
            }
            None => fail(format!("{} has no field '{}'", o.def.decl.name, name)),
        },
        Value::Module(m) => fail(format!("cannot assign to '{}' of module {}", name, m.name)),
        other => fail(format!("cannot set field '{}' on {}", name, other.type_name())),
    }
}

fn index_of(i: i64, len: usize) -> R<usize> {
    if i < 0 || i as usize >= len {
        return fail(format!("index {} is out of range (length {})", i, len));
    }
    Ok(i as usize)
}

fn range_of(a: i64, b: i64, len: usize) -> R<(usize, usize)> {
    if a < 0 || b < a || b as usize > len {
        return fail(format!("range {} .. {} is out of bounds (length {})", a, b, len));
    }
    Ok((a as usize, b as usize))
}

fn get_index(target: &Value, key: &Value) -> R<Value> {
    match (target, key) {
        (Value::Array(a), Value::Int(i)) => {
            let items = a.borrow();
            Ok(items[index_of(*i, items.len())?].clone())
        }
        (Value::Array(a), Value::Range(x, y)) => {
            let items = a.borrow();
            let (s, e) = range_of(*x, *y, items.len())?;
            Ok(Value::array(items[s..e].to_vec()))
        }
        (Value::Str(s), Value::Int(i)) => {
            let n = s.char_len();
            let i = index_of(*i, n)?;
            Ok(Value::str(s.slice(i, i + 1)))
        }
        (Value::Str(s), Value::Range(x, y)) => {
            let (a, b) = range_of(*x, *y, s.char_len())?;
            Ok(Value::str(s.slice(a, b)))
        }
        (Value::Dict(d), key) => match d.borrow().iter().find(|(k, _)| values_equal(k, key)) {
            Some((_, v)) => Ok(v.clone()),
            None => fail(format!("key {} not found in dictionary", to_text(key))),
        },
        (t, k) => fail(format!("cannot index {} with {}", t.type_name(), k.type_name())),
    }
}

fn set_index(target: &Value, key: Value, value: Value) -> R<()> {
    match (target, &key) {
        (Value::Array(a), Value::Int(i)) => {
            let mut items = a.borrow_mut();
            let idx = index_of(*i, items.len())?;
            items[idx] = value;
            Ok(())
        }
        (Value::Dict(d), _) => {
            let mut entries = d.borrow_mut();
            match entries.iter_mut().find(|(k, _)| values_equal(k, &key)) {
                Some(entry) => entry.1 = value,
                None => entries.push((key, value)),
            }
            Ok(())
        }
        (Value::Str(_), _) => fail("strings are immutable"),
        (t, k) => fail(format!("cannot index {} with {}", t.type_name(), k.type_name())),
    }
}

fn compare(a: &Value, b: &Value) -> R<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(x.cmp(y)),
        (Value::Str(x), Value::Str(y)) => Ok(x.text.cmp(&y.text)),
        _ => {
            let (x, y) = match (as_float(a), as_float(b)) {
                (Some(x), Some(y)) => (x, y),
                _ => return fail(format!("cannot compare {} with {}", a.type_name(), b.type_name())),
            };
            Ok(x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal))
        }
    }
}

fn as_float(v: &Value) -> Option<f64> {
    match v {
        Value::Int(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn binary(op: &str, l: &Value, r: &Value) -> R<Value> {
    use std::cmp::Ordering::*;
    match op {
        ".:" => Ok(Value::str(to_text(l) + &to_text(r))),
        "==" => Ok(Value::Bool(values_equal(l, r))),
        "!=" => Ok(Value::Bool(!values_equal(l, r))),
        "<" => Ok(Value::Bool(compare(l, r)? == Less)),
        "<=" => Ok(Value::Bool(compare(l, r)? != Greater)),
        ">" => Ok(Value::Bool(compare(l, r)? == Greater)),
        ">=" => Ok(Value::Bool(compare(l, r)? != Less)),
        ".." => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Range(*a, *b)),
            _ => fail("'..' needs two numbers"),
        },
        "+" | "-" | "*" | "/" | "%" => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(match op {
                "+" => a.wrapping_add(*b),
                "-" => a.wrapping_sub(*b),
                "*" => a.wrapping_mul(*b),
                "/" => {
                    if *b == 0 {
                        return fail("division by zero");
                    }
                    a.wrapping_div(*b)
                }
                _ => {
                    if *b == 0 {
                        return fail("division by zero");
                    }
                    a.wrapping_rem(*b)
                }
            })),
            (Value::Str(_), _) | (_, Value::Str(_)) if op == "+" => {
                fail("'+' does not join text, use '.:'")
            }
            _ => match (as_float(l), as_float(r)) {
                (Some(a), Some(b)) => Ok(Value::Float(match op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b,
                    _ => a % b,
                })),
                _ => fail(format!("operator '{}' cannot be applied to {} and {}", op, l.type_name(), r.type_name())),
            },
        },
        "&" | "|" | "^" | "shl" | "shr" => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(match op {
                "&" => a & b,
                "|" => a | b,
                "^" => a ^ b,
                "shl" => a.wrapping_shl(*b as u32),
                _ => a.wrapping_shr(*b as u32),
            })),
            _ => fail(format!("operator '{}' needs two numbers", op)),
        },
        other => fail(format!("unknown operator '{}'", other)),
    }
}
