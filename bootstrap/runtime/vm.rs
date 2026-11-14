//! `clpp-vm`: register-based virtual machine for CL++ bytecode (`.clpb`).
//!
//! The bytecode is text so it can be read and diffed:
//!
//! ```text
//! .clpb 1
//! .module Main
//! .enum Color Red Green
//! .def Point X Y
//! .global count
//! .func Point.Sum 2 4        ; name, parameters (r0 = this for methods), registers
//! .line 7
//!   GETF r2 r0 X
//!   ADD r3 r2 r1
//!   RET r3
//! .end
//! ```
//!
//! Frames live on an explicit stack (no Rust recursion), registers of all
//! frames share one vector, and field access / method calls use inline caches.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use crate::stdlib::{self, SVal};

// ==================================================================== values

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Array(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<Vec<(Value, Value)>>>),
    Object(Rc<Object>),
    Enum(u32, u32),
    Range(i64, i64),
}

pub struct Object {
    def: u32,
    fields: RefCell<Vec<Value>>,
}

impl Value {
    fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }

    fn type_name(&self, p: &Program) -> String {
        match self {
            Value::Null => "null".into(),
            Value::Bool(_) => "bool".into(),
            Value::Int(_) => "number".into(),
            Value::Float(_) => "float".into(),
            Value::Str(_) => "string".into(),
            Value::Array(_) => "array".into(),
            Value::Dict(_) => "dictionary".into(),
            Value::Object(o) => plain(&p.defs[o.def as usize].name).to_string(),
            Value::Enum(e, _) => plain(&p.enums[*e as usize].name).to_string(),
            Value::Range(..) => "range".into(),
        }
    }
}

fn char_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    }
}

fn char_slice(s: &str, a: usize, b: usize) -> String {
    if s.is_ascii() {
        s[a..b].to_string()
    } else {
        s.chars().skip(a).take(b - a).collect()
    }
}

fn display(p: &Program, v: &Value, out: &mut String, depth: usize) {
    match v {
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
                let _ = write!(out, "{:?}", &**s);
            } else {
                out.push_str(s);
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
                display(p, item, out, depth + 1);
            }
            out.push('}');
        }
        Value::Dict(entries) => {
            if depth > 4 {
                out.push_str("{...}");
                return;
            }
            out.push('{');
            for (i, (k, val)) in entries.borrow().iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push('[');
                display(p, k, out, depth + 1);
                out.push_str("] = ");
                display(p, val, out, depth + 1);
            }
            out.push('}');
        }
        Value::Object(o) => {
            let _ = write!(out, "{} {{...}}", plain(&p.defs[o.def as usize].name));
        }
        Value::Enum(e, i) => {
            let info = &p.enums[*e as usize];
            let _ = write!(out, "{}::{}", plain(&info.name), info.variants[*i as usize]);
        }
        Value::Range(a, b) => {
            let _ = write!(out, "{} .. {}", a, b);
        }
    }
}

/// `ast$Node` -> `Node`: names of linked modules carry a prefix.
fn plain(name: &str) -> &str {
    name.rsplit('$').next().unwrap_or(name)
}

fn text(p: &Program, v: &Value) -> String {
    let mut out = String::new();
    display(p, v, &mut out, 0);
    out
}

fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => (*x as f64) == *y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Enum(e1, i1), Value::Enum(e2, i2)) => e1 == e2 && i1 == i2,
        (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
        (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
        (Value::Dict(x), Value::Dict(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

// ==================================================================== program

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Cat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    BAnd,
    BOr,
    BXor,
    Shl,
    Shr,
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Un {
    Not,
    Neg,
    BNot,
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
    /// `@clpp.<module>` member, e.g. ("math", "sqrt").
    Native(&'static str, &'static str),
}

pub enum Ins {
    Move(u32, u32),
    LoadI(u32, i64),
    LoadF(u32, f64),
    LoadS(u32, Rc<str>),
    LoadB(u32, bool),
    LoadN(u32),
    GetG(u32, u32),
    SetG(u32, u32),
    Bin(Op, u32, u32, u32),
    Unary(Un, u32, u32),
    Jmp(u32),
    Jt(u32, u32),
    Jf(u32, u32),
    Call(u32, u32, u32, u32),
    CallB(u32, Builtin, u32, u32),
    /// dst, receiver, method name, args base, arg count, cache (def, function).
    CallM(u32, u32, Rc<str>, u32, u32, Cell<(u32, u32)>),
    Ret(u32),
    RetN,
    NewArr(u32, u32, u32),
    NewDict(u32, u32, u32),
    Index(u32, u32, u32),
    SetIndex(u32, u32, u32),
    New(u32, u32),
    /// dst, object, field name, cache (def, slot).
    GetF(u32, u32, Rc<str>, Cell<(u32, u32)>),
    SetF(u32, Rc<str>, u32, Cell<(u32, u32)>),
    Enum(u32, u32, u32),
    Try(u32, u32),
    EndTry,
    IterList(u32, u32),
    Fail(Rc<str>),
    /// dst, src, type name, exact: `(Type value)` and typed variables.
    Conv(u32, u32, Rc<str>, bool),
}

pub struct Func {
    pub name: String,
    /// Source file of the module this function came from.
    pub source: String,
    pub params: u32,
    pub regs: u32,
    pub code: Vec<Ins>,
    pub lines: Vec<u32>,
}

pub struct Def {
    pub name: String,
    pub fields: Vec<String>,
    /// Numeric type of each field (`X:f32` in `.def`), enforced on SETF.
    pub field_types: Vec<Option<String>>,
    pub methods: HashMap<String, u32>,
}

pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<String>,
}

pub struct Program {
    pub module: String,
    /// Source file the bytecode was compiled from (`.source`), for errors.
    pub source: String,
    pub funcs: Vec<Func>,
    pub defs: Vec<Def>,
    pub enums: Vec<EnumInfo>,
    pub globals: Vec<String>,
}

const NONE: u32 = u32::MAX;

// ==================================================================== loader

fn split_operands(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == ';' {
            break;
        }
        if c == '"' || c == '\'' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err("unterminated string operand".into());
            }
            i += 1;
            out.push(chars[start..i].iter().collect());
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        out.push(chars[start..i].iter().collect());
    }
    Ok(out)
}

fn unquote(lit: &str) -> Result<String, String> {
    let inner: Vec<char> = lit.chars().collect();
    if inner.len() < 2 {
        return Err(format!("bad string literal {}", lit));
    }
    let mut out = String::new();
    let mut i = 1;
    while i < inner.len() - 1 {
        let c = inner[i];
        if c == '\\' && i + 1 < inner.len() - 1 {
            i += 1;
            out.push(match inner[i] {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '0' => '\0',
                other => other,
            });
        } else {
            out.push(c);
        }
        i += 1;
    }
    Ok(out)
}

fn parse_int(text: &str) -> Option<i64> {
    let clean: String = text.chars().filter(|c| *c != '_').collect();
    if let Some(h) = clean.strip_prefix("0x").or(clean.strip_prefix("0X")) {
        return u64::from_str_radix(h, 16).ok().map(|v| v as i64);
    }
    if let Some(b) = clean.strip_prefix("0b").or(clean.strip_prefix("0B")) {
        return u64::from_str_radix(b, 2).ok().map(|v| v as i64);
    }
    clean.parse::<i64>().ok()
}

fn reg(t: &str) -> Result<u32, String> {
    t.strip_prefix('r').and_then(|n| n.parse().ok()).ok_or_else(|| format!("expected register, got '{}'", t))
}

fn num(t: &str) -> Result<u32, String> {
    t.parse().map_err(|_| format!("expected number, got '{}'", t))
}

fn builtin(name: &str) -> Option<Builtin> {
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
        other => {
            let (module, member) = other.split_once('.')?;
            let (m, n, _, _) = stdlib::CATALOG.iter().find(|(m, n, ..)| *m == module && *n == member)?;
            Builtin::Native(m, n)
        }
    })
}

fn bin_op(name: &str) -> Option<Op> {
    Some(match name {
        "ADD" => Op::Add,
        "SUB" => Op::Sub,
        "MUL" => Op::Mul,
        "DIV" => Op::Div,
        "MOD" => Op::Mod,
        "CAT" => Op::Cat,
        "EQ" => Op::Eq,
        "NE" => Op::Ne,
        "LT" => Op::Lt,
        "LE" => Op::Le,
        "GT" => Op::Gt,
        "GE" => Op::Ge,
        "BAND" => Op::BAnd,
        "BOR" => Op::BOr,
        "BXOR" => Op::BXor,
        "SHL" => Op::Shl,
        "SHR" => Op::Shr,
        "RANGE" => Op::Range,
        _ => return None,
    })
}

pub fn load(source: &str) -> Result<Program, String> {
    let mut program = Program { module: String::new(), source: String::new(), funcs: Vec::new(), defs: Vec::new(), enums: Vec::new(), globals: Vec::new() };
    // Raw function bodies; names are resolved after every declaration is known.
    struct RawFunc {
        name: String,
        source: String,
        params: u32,
        regs: u32,
        lines: Vec<(usize, u32, Vec<String>)>,
        labels: HashMap<String, u32>,
    }
    let mut raw: Vec<RawFunc> = Vec::new();
    let mut current: Option<RawFunc> = None;
    let mut source_line = 0u32;
    let mut current_source = String::new();
    let mut saw_header = false;

    for (number, line) in source.lines().enumerate() {
        let number = number + 1;
        let err = |m: String| format!("bytecode line {}: {}", number, m);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        let parts = split_operands(trimmed).map_err(err)?;
        if parts.is_empty() {
            continue;
        }
        let head = parts[0].as_str();
        match head {
            ".clpb" => {
                if parts.get(1).map(String::as_str) != Some("1") {
                    return Err(err("unsupported bytecode version".into()));
                }
                saw_header = true;
            }
            ".module" => program.module = parts.get(1).cloned().unwrap_or_default(),
            ".source" => {
                current_source = trimmed[".source".len()..].trim().to_string();
                if program.source.is_empty() {
                    program.source = current_source.clone();
                }
            }
            ".enum" => program.enums.push(EnumInfo { name: parts[1].clone(), variants: parts[2..].to_vec() }),
            ".def" => {
                let mut fields = Vec::new();
                let mut field_types = Vec::new();
                for f in &parts[2..] {
                    match f.split_once(':') {
                        Some((n, t)) => {
                            fields.push(n.to_string());
                            field_types.push(Some(t.to_string()));
                        }
                        None => {
                            fields.push(f.clone());
                            field_types.push(None);
                        }
                    }
                }
                program.defs.push(Def { name: parts[1].clone(), fields, field_types, methods: HashMap::new() })
            }
            ".global" => program.globals.push(parts[1].clone()),
            ".func" => {
                if current.is_some() {
                    return Err(err("'.func' inside another function".into()));
                }
                if parts.len() != 4 {
                    return Err(err("expected .func <name> <params> <registers>".into()));
                }
                current = Some(RawFunc {
                    name: parts[1].clone(),
                    source: current_source.clone(),
                    params: num(&parts[2]).map_err(err)?,
                    regs: num(&parts[3]).map_err(err)?,
                    lines: Vec::new(),
                    labels: HashMap::new(),
                });
            }
            ".line" => source_line = num(parts.get(1).map(String::as_str).unwrap_or("0")).map_err(err)?,
            ".end" => match current.take() {
                Some(f) => raw.push(f),
                None => return Err(err("'.end' without '.func'".into())),
            },
            _ => {
                let f = current.as_mut().ok_or_else(|| err("instruction outside of a function".into()))?;
                if let Some(label) = head.strip_suffix(':') {
                    f.labels.insert(label.to_string(), f.lines.len() as u32);
                } else {
                    f.lines.push((number, source_line, parts));
                }
            }
        }
    }
    if !saw_header {
        return Err("not CL++ bytecode: missing '.clpb 1' header".into());
    }
    if current.is_some() {
        return Err("missing '.end' at end of file".into());
    }

    let func_index: HashMap<String, u32> = raw.iter().enumerate().map(|(i, f)| (f.name.clone(), i as u32)).collect();
    let def_index: HashMap<String, u32> = program.defs.iter().enumerate().map(|(i, d)| (d.name.clone(), i as u32)).collect();
    let enum_index: HashMap<String, u32> = program.enums.iter().enumerate().map(|(i, e)| (e.name.clone(), i as u32)).collect();
    let global_index: HashMap<String, u32> = program.globals.iter().enumerate().map(|(i, g)| (g.clone(), i as u32)).collect();

    for (i, f) in raw.iter().enumerate() {
        if let Some((owner, method)) = f.name.split_once('.') {
            if let Some(&d) = def_index.get(owner) {
                program.defs[d as usize].methods.insert(method.to_string(), i as u32);
            }
        }
    }

    for f in raw {
        let mut code = Vec::with_capacity(f.lines.len());
        let mut lines = Vec::with_capacity(f.lines.len());
        for (number, src_line, parts) in &f.lines {
            let err = |m: String| format!("bytecode line {}: {}", number, m);
            let arg = |i: usize| parts.get(i).map(String::as_str).ok_or_else(|| err(format!("{} needs more operands", parts[0])));
            let label = |i: usize| -> Result<u32, String> {
                let name = arg(i)?;
                f.labels.get(name).copied().ok_or_else(|| err(format!("unknown label '{}'", name)))
            };
            let op = parts[0].as_str();
            let ins = if let Some(o) = bin_op(op) {
                Ins::Bin(o, reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?, reg(arg(3)?).map_err(err)?)
            } else {
                match op {
                    "MOVE" => Ins::Move(reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?),
                    "LOADI" => Ins::LoadI(reg(arg(1)?).map_err(err)?, parse_int(arg(2)?).ok_or_else(|| err("bad integer".into()))?),
                    "LOADF" => Ins::LoadF(
                        reg(arg(1)?).map_err(err)?,
                        arg(2)?.replace('_', "").parse().map_err(|_| err("bad float".into()))?,
                    ),
                    "LOADS" => Ins::LoadS(reg(arg(1)?).map_err(err)?, Rc::from(unquote(arg(2)?).map_err(err)?.as_str())),
                    "LOADB" => Ins::LoadB(reg(arg(1)?).map_err(err)?, arg(2)? == "1"),
                    "LOADN" => Ins::LoadN(reg(arg(1)?).map_err(err)?),
                    "GETG" | "SETG" => {
                        let (r, g) = if op == "GETG" { (arg(1)?, arg(2)?) } else { (arg(2)?, arg(1)?) };
                        let gi = *global_index.get(g).ok_or_else(|| err(format!("unknown global '{}'", g)))?;
                        let r = reg(r).map_err(err)?;
                        if op == "GETG" {
                            Ins::GetG(r, gi)
                        } else {
                            Ins::SetG(gi, r)
                        }
                    }
                    "NOT" | "NEG" | "BNOT" => Ins::Unary(
                        match op {
                            "NOT" => Un::Not,
                            "NEG" => Un::Neg,
                            _ => Un::BNot,
                        },
                        reg(arg(1)?).map_err(err)?,
                        reg(arg(2)?).map_err(err)?,
                    ),
                    "JMP" => Ins::Jmp(label(1)?),
                    "JT" => Ins::Jt(reg(arg(1)?).map_err(err)?, label(2)?),
                    "JF" => Ins::Jf(reg(arg(1)?).map_err(err)?, label(2)?),
                    "CALL" => {
                        let name = arg(2)?;
                        let fi = *func_index.get(name).ok_or_else(|| err(format!("unknown function '{}'", name)))?;
                        Ins::Call(reg(arg(1)?).map_err(err)?, fi, reg(arg(3)?).map_err(err)?, num(arg(4)?).map_err(err)?)
                    }
                    "CALLB" => {
                        let name = arg(2)?;
                        let b = builtin(name).ok_or_else(|| err(format!("unknown built-in '{}'", name)))?;
                        Ins::CallB(reg(arg(1)?).map_err(err)?, b, reg(arg(3)?).map_err(err)?, num(arg(4)?).map_err(err)?)
                    }
                    "CALLM" => Ins::CallM(
                        reg(arg(1)?).map_err(err)?,
                        reg(arg(2)?).map_err(err)?,
                        Rc::from(arg(3)?),
                        reg(arg(4)?).map_err(err)?,
                        num(arg(5)?).map_err(err)?,
                        Cell::new((NONE, NONE)),
                    ),
                    "RET" => Ins::Ret(reg(arg(1)?).map_err(err)?),
                    "RETN" => Ins::RetN,
                    "NEWARR" => Ins::NewArr(reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?, num(arg(3)?).map_err(err)?),
                    "NEWDICT" => Ins::NewDict(reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?, num(arg(3)?).map_err(err)?),
                    "INDEX" => Ins::Index(reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?, reg(arg(3)?).map_err(err)?),
                    "SETINDEX" => Ins::SetIndex(reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?, reg(arg(3)?).map_err(err)?),
                    "NEW" => {
                        let name = arg(2)?;
                        let d = *def_index.get(name).ok_or_else(|| err(format!("unknown def '{}'", name)))?;
                        Ins::New(reg(arg(1)?).map_err(err)?, d)
                    }
                    "GETF" => Ins::GetF(
                        reg(arg(1)?).map_err(err)?,
                        reg(arg(2)?).map_err(err)?,
                        Rc::from(arg(3)?),
                        Cell::new((NONE, NONE)),
                    ),
                    "SETF" => Ins::SetF(
                        reg(arg(1)?).map_err(err)?,
                        Rc::from(arg(2)?),
                        reg(arg(3)?).map_err(err)?,
                        Cell::new((NONE, NONE)),
                    ),
                    "ENUM" => {
                        let e = *enum_index.get(arg(2)?).ok_or_else(|| err(format!("unknown enum '{}'", arg(2).unwrap_or(""))))?;
                        let v = program.enums[e as usize]
                            .variants
                            .iter()
                            .position(|v| v == arg(3).unwrap_or(""))
                            .ok_or_else(|| err(format!("enum class {} has no member '{}'", program.enums[e as usize].name, arg(3).unwrap_or(""))))?;
                        Ins::Enum(reg(arg(1)?).map_err(err)?, e, v as u32)
                    }
                    "TRY" => Ins::Try(reg(arg(1)?).map_err(err)?, label(2)?),
                    "ENDTRY" => Ins::EndTry,
                    "ITERLIST" => Ins::IterList(reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?),
                    "FAIL" => Ins::Fail(Rc::from(unquote(arg(1)?).map_err(err)?.as_str())),
                    "CONV" => Ins::Conv(reg(arg(1)?).map_err(err)?, reg(arg(2)?).map_err(err)?, Rc::from(arg(3)?), arg(4)? == "1"),
                    other => return Err(err(format!("unknown instruction '{}'", other))),
                }
            };
            code.push(ins);
            lines.push(*src_line);
        }
        if f.regs < f.params {
            return Err(format!("function {} has fewer registers than parameters", f.name));
        }
        program.funcs.push(Func { name: f.name, source: f.source, params: f.params, regs: f.regs, code, lines });
    }
    if !program.funcs.iter().any(|f| f.name == "__main") {
        return Err("bytecode has no __main function".into());
    }
    Ok(program)
}

// ==================================================================== machine

struct Frame {
    func: u32,
    pc: usize,
    base: usize,
    /// Absolute register in the caller that receives the result.
    ret: usize,
}

struct Handler {
    depth: usize,
    catch_pc: u32,
    err_reg: usize,
}

pub struct Vm<'p> {
    p: &'p Program,
    regs: Vec<Value>,
    frames: Vec<Frame>,
    handlers: Vec<Handler>,
    globals: Vec<Value>,
    args: Vec<String>,
}

const MAX_FRAMES: usize = 200_000;

enum Step {
    Continue,
    Error(String),
}

impl<'p> Vm<'p> {
    pub fn new(p: &'p Program, args: Vec<String>) -> Vm<'p> {
        Vm { p, regs: Vec::new(), frames: Vec::new(), handlers: Vec::new(), globals: vec![Value::Null; p.globals.len()], args }
    }

    /// Innermost frame first: (function, source line, source file).
    fn frames_info(&self) -> Vec<(String, u32, String)> {
        self.frames
            .iter()
            .rev()
            .map(|f| {
                let func = &self.p.funcs[f.func as usize];
                (func.name.clone(), func.lines.get(f.pc.saturating_sub(1)).copied().unwrap_or(0), func.source.clone())
            })
            .collect()
    }

    pub fn run(&mut self) -> Result<(), VmError> {
        let main = self.p.funcs.iter().position(|f| f.name == "__main").unwrap() as u32;
        self.regs.resize(self.p.funcs[main as usize].regs as usize + 1, Value::Null);
        self.frames.push(Frame { func: main, pc: 0, base: 1, ret: 0 });
        loop {
            match self.execute() {
                Ok(()) => return Ok(()),
                Err(message) => {
                    // Unwind to the innermost try handler, if any.
                    match self.handlers.pop() {
                        Some(h) => {
                            self.frames.truncate(h.depth);
                            let frame = self.frames.last_mut().unwrap();
                            frame.pc = h.catch_pc as usize;
                            let top = frame.base + self.p.funcs[frame.func as usize].regs as usize;
                            self.regs.truncate(top);
                            self.regs[h.err_reg] = Value::str(&message);
                        }
                        None => return Err(VmError { message, frames: self.frames_info() }),
                    }
                }
            }
        }
    }

    fn fail<T>(&self, message: String) -> Result<T, String> {
        Err(message)
    }

    fn execute(&mut self) -> Result<(), String> {
        let p = self.p;
        loop {
            let frame = self.frames.last_mut().unwrap();
            let func = &p.funcs[frame.func as usize];
            let base = frame.base;
            let ins = match func.code.get(frame.pc) {
                Some(i) => i,
                None => {
                    // Falling off the end returns null.
                    if self.do_return(Value::Null) {
                        return Ok(());
                    }
                    continue;
                }
            };
            frame.pc += 1;
            let r = |i: u32| base + i as usize;
            match ins {
                Ins::Move(a, b) => {
                    let v = self.regs[r(*b)].clone();
                    self.regs[r(*a)] = v;
                }
                Ins::LoadI(a, i) => self.regs[r(*a)] = Value::Int(*i),
                Ins::LoadF(a, f) => self.regs[r(*a)] = Value::Float(*f),
                Ins::LoadS(a, s) => self.regs[r(*a)] = Value::Str(s.clone()),
                Ins::LoadB(a, b) => self.regs[r(*a)] = Value::Bool(*b),
                Ins::LoadN(a) => self.regs[r(*a)] = Value::Null,
                Ins::GetG(a, g) => self.regs[r(*a)] = self.globals[*g as usize].clone(),
                Ins::SetG(g, a) => self.globals[*g as usize] = self.regs[r(*a)].clone(),
                Ins::Bin(op, a, b, c) => {
                    let v = binary(p, *op, &self.regs[r(*b)], &self.regs[r(*c)])?;
                    self.regs[r(*a)] = v;
                }
                Ins::Unary(op, a, b) => {
                    let v = match (op, &self.regs[r(*b)]) {
                        (Un::Not, Value::Bool(x)) => Value::Bool(!x),
                        (Un::Neg, Value::Int(x)) => Value::Int(x.wrapping_neg()),
                        (Un::Neg, Value::Float(x)) => Value::Float(-x),
                        (Un::BNot, Value::Int(x)) => Value::Int(!x),
                        (op, v) => return self.fail(format!("operator {:?} cannot be applied to {}", op, v.type_name(p))),
                    };
                    self.regs[r(*a)] = v;
                }
                Ins::Jmp(t) => self.frames.last_mut().unwrap().pc = *t as usize,
                Ins::Jt(a, t) | Ins::Jf(a, t) => {
                    let want = matches!(ins, Ins::Jt(..));
                    match &self.regs[r(*a)] {
                        Value::Bool(b) => {
                            if *b == want {
                                self.frames.last_mut().unwrap().pc = *t as usize;
                            }
                        }
                        v => return self.fail(format!("condition must be bool, got {}", v.type_name(p))),
                    }
                }
                Ins::Call(a, f, b, n) => {
                    let args: Vec<Value> = self.regs[r(*b)..r(*b) + *n as usize].to_vec();
                    self.push_frame(*f, args, r(*a))?;
                }
                Ins::CallM(a, recv, name, b, n, cache) => {
                    let obj = match &self.regs[r(*recv)] {
                        Value::Object(o) => o.clone(),
                        v => return self.fail(format!("{} has no method '{}'", v.type_name(p), name)),
                    };
                    let (cdef, cfunc) = cache.get();
                    let f = if cdef == obj.def {
                        cfunc
                    } else {
                        let def = &p.defs[obj.def as usize];
                        let f = *def
                            .methods
                            .get(&**name)
                            .ok_or_else(|| format!("{} has no method '{}'", def.name, name))?;
                        cache.set((obj.def, f));
                        f
                    };
                    let mut args = Vec::with_capacity(*n as usize + 1);
                    args.push(Value::Object(obj));
                    args.extend_from_slice(&self.regs[r(*b)..r(*b) + *n as usize]);
                    self.push_frame(f, args, r(*a))?;
                }
                Ins::CallB(a, b, base_reg, n) => {
                    let args: Vec<Value> = self.regs[r(*base_reg)..r(*base_reg) + *n as usize].to_vec();
                    let v = self.builtin(*b, args)?;
                    self.regs[r(*a)] = v;
                }
                Ins::Ret(a) => {
                    let v = self.regs[r(*a)].clone();
                    if self.do_return(v) {
                        return Ok(());
                    }
                }
                Ins::RetN => {
                    if self.do_return(Value::Null) {
                        return Ok(());
                    }
                }
                Ins::NewArr(a, b, n) => {
                    let items = self.regs[r(*b)..r(*b) + *n as usize].to_vec();
                    self.regs[r(*a)] = Value::Array(Rc::new(RefCell::new(items)));
                }
                Ins::NewDict(a, b, n) => {
                    let mut entries: Vec<(Value, Value)> = Vec::with_capacity(*n as usize);
                    for i in 0..*n as usize {
                        let k = self.regs[r(*b) + 2 * i].clone();
                        let v = self.regs[r(*b) + 2 * i + 1].clone();
                        if entries.iter().any(|(e, _)| equal(e, &k)) {
                            return self.fail(format!("duplicate dictionary key {}", text(p, &k)));
                        }
                        entries.push((k, v));
                    }
                    self.regs[r(*a)] = Value::Dict(Rc::new(RefCell::new(entries)));
                }
                Ins::Index(a, b, c) => {
                    let v = get_index(p, &self.regs[r(*b)], &self.regs[r(*c)])?;
                    self.regs[r(*a)] = v;
                }
                Ins::SetIndex(a, b, c) => {
                    let value = self.regs[r(*c)].clone();
                    set_index(p, &self.regs[r(*a)], self.regs[r(*b)].clone(), value)?;
                }
                Ins::New(a, d) => {
                    let n = p.defs[*d as usize].fields.len();
                    self.regs[r(*a)] = Value::Object(Rc::new(Object { def: *d, fields: RefCell::new(vec![Value::Null; n]) }));
                }
                Ins::GetF(a, b, name, cache) => {
                    let v = match &self.regs[r(*b)] {
                        Value::Object(o) => {
                            let slot = field_slot(p, o, name, cache)?;
                            o.fields.borrow()[slot].clone()
                        }
                        v => return self.fail(format!("{} has no field '{}'", v.type_name(p), name)),
                    };
                    self.regs[r(*a)] = v;
                }
                Ins::SetF(a, name, b, cache) => {
                    let value = self.regs[r(*b)].clone();
                    match &self.regs[r(*a)] {
                        Value::Object(o) => {
                            let slot = field_slot(p, o, name, cache)?;
                            let value = match &p.defs[o.def as usize].field_types[slot] {
                                Some(t) => convert(p, &value, t, false)?,
                                None => value,
                            };
                            o.fields.borrow_mut()[slot] = value;
                        }
                        v => return self.fail(format!("cannot set field '{}' on {}", name, v.type_name(p))),
                    }
                }
                Ins::Enum(a, e, v) => self.regs[r(*a)] = Value::Enum(*e, *v),
                Ins::Try(e, t) => {
                    let depth = self.frames.len();
                    self.handlers.push(Handler { depth, catch_pc: *t, err_reg: r(*e) });
                }
                Ins::EndTry => {
                    self.handlers.pop();
                }
                Ins::IterList(a, b) => {
                    let items: Vec<Value> = match &self.regs[r(*b)] {
                        Value::Array(x) => x.borrow().clone(),
                        Value::Dict(d) => d.borrow().iter().map(|(k, _)| k.clone()).collect(),
                        Value::Str(s) => s.chars().map(|c| Value::str(&c.to_string())).collect(),
                        Value::Range(x, y) => (*x..*y).map(Value::Int).collect(),
                        v => return self.fail(format!("cannot iterate over {}", v.type_name(p))),
                    };
                    self.regs[r(*a)] = Value::Array(Rc::new(RefCell::new(items)));
                }
                Ins::Fail(m) => return self.fail(m.to_string()),
                Ins::Conv(a, b, ty, exact) => {
                    let v = convert(p, &self.regs[r(*b)], ty, *exact)?;
                    self.regs[r(*a)] = v;
                }
            }
        }
    }

    fn push_frame(&mut self, f: u32, args: Vec<Value>, ret: usize) -> Result<(), String> {
        let func = &self.p.funcs[f as usize];
        if args.len() != func.params as usize {
            return self.fail(format!("{} expects {} argument(s), got {}", func.name, func.params, args.len()));
        }
        if self.frames.len() >= MAX_FRAMES {
            return self.fail(format!("call depth limit ({}) exceeded in {}", MAX_FRAMES, func.name));
        }
        let caller = self.frames.last().unwrap();
        let base = caller.base + self.p.funcs[caller.func as usize].regs as usize;
        let top = base + func.regs as usize;
        if self.regs.len() < top {
            self.regs.resize(top, Value::Null);
        }
        for (i, v) in args.into_iter().enumerate() {
            self.regs[base + i] = v;
        }
        for i in func.params as usize..func.regs as usize {
            self.regs[base + i] = Value::Null;
        }
        self.frames.push(Frame { func: f, pc: 0, base, ret });
        Ok(())
    }

    /// Returns true when the outermost frame returned.
    fn do_return(&mut self, v: Value) -> bool {
        let frame = self.frames.pop().unwrap();
        let depth = self.frames.len();
        while self.handlers.last().map(|h| h.depth > depth).unwrap_or(false) {
            self.handlers.pop();
        }
        if self.frames.is_empty() {
            return true;
        }
        self.regs[frame.ret] = v;
        false
    }

    fn builtin(&mut self, b: Builtin, args: Vec<Value>) -> Result<Value, String> {
        let p = self.p;
        let arity = |n: usize, name: &str| -> Result<(), String> {
            if args.len() != n {
                return Err(format!("{} expects {} argument(s), got {}", name, n, args.len()));
            }
            Ok(())
        };
        match b {
            Builtin::Post => {
                arity(1, "post")?;
                println!("{}", text(p, &args[0]));
                Ok(Value::Null)
            }
            Builtin::Warn => {
                arity(1, "warn")?;
                eprintln!("warning: {}", text(p, &args[0]));
                Ok(Value::Null)
            }
            Builtin::Report => {
                arity(1, "report")?;
                Err(text(p, &args[0]))
            }
            Builtin::Len => {
                arity(1, "len")?;
                Ok(Value::Int(match &args[0] {
                    Value::Str(s) => char_len(s) as i64,
                    Value::Array(a) => a.borrow().len() as i64,
                    Value::Dict(d) => d.borrow().len() as i64,
                    v => return Err(format!("len() needs a string or collection, got {}", v.type_name(p))),
                }))
            }
            Builtin::Push => {
                if args.len() < 2 {
                    return Err("push expects an array and at least one value".into());
                }
                match &args[0] {
                    Value::Array(a) => {
                        a.borrow_mut().extend(args[1..].iter().cloned());
                        Ok(Value::Null)
                    }
                    v => Err(format!("push() needs an array, got {}", v.type_name(p))),
                }
            }
            Builtin::Pop => {
                arity(1, "pop")?;
                match &args[0] {
                    Value::Array(a) => a.borrow_mut().pop().ok_or_else(|| "pop() on an empty array".to_string()),
                    v => Err(format!("pop() needs an array, got {}", v.type_name(p))),
                }
            }
            Builtin::Insert => {
                arity(3, "insert")?;
                match (&args[0], &args[1]) {
                    (Value::Array(a), Value::Int(i)) => {
                        let mut items = a.borrow_mut();
                        if *i < 0 || *i as usize > items.len() {
                            return Err(format!("insert index {} is out of range (length {})", i, items.len()));
                        }
                        items.insert(*i as usize, args[2].clone());
                        Ok(Value::Null)
                    }
                    _ => Err("insert() expects (array, number, value)".into()),
                }
            }
            Builtin::Remove => {
                arity(2, "remove")?;
                match (&args[0], &args[1]) {
                    (Value::Array(a), Value::Int(i)) => {
                        let mut items = a.borrow_mut();
                        if *i < 0 || *i as usize >= items.len() {
                            return Err(format!("remove index {} is out of range (length {})", i, items.len()));
                        }
                        Ok(items.remove(*i as usize))
                    }
                    (Value::Dict(d), key) => {
                        let mut entries = d.borrow_mut();
                        match entries.iter().position(|(k, _)| equal(k, key)) {
                            Some(i) => Ok(entries.remove(i).1),
                            None => Err(format!("key {} not found", text(p, key))),
                        }
                    }
                    _ => Err("remove() expects (array, number) or (dictionary, key)".into()),
                }
            }
            Builtin::Find => {
                arity(2, "find")?;
                match (&args[0], &args[1]) {
                    (Value::Array(a), needle) => Ok(Value::Int(
                        a.borrow().iter().position(|v| equal(v, needle)).map(|i| i as i64).unwrap_or(-1),
                    )),
                    (Value::Str(s), Value::Str(n)) => Ok(Value::Int(match s.find(&**n) {
                        Some(byte) => s[..byte].chars().count() as i64,
                        None => -1,
                    })),
                    (Value::Dict(d), key) => Ok(Value::Bool(d.borrow().iter().any(|(k, _)| equal(k, key)))),
                    _ => Err("find() expects (array, value), (string, string) or (dictionary, key)".into()),
                }
            }
            Builtin::Sort => {
                arity(1, "sort")?;
                match &args[0] {
                    Value::Array(a) => {
                        let mut items = a.borrow_mut();
                        let mut bad = false;
                        items.sort_by(|x, y| match compare(x, y) {
                            Some(o) => o,
                            None => {
                                bad = true;
                                std::cmp::Ordering::Equal
                            }
                        });
                        if bad {
                            return Err("sort() needs an array of numbers or of strings".into());
                        }
                        Ok(Value::Null)
                    }
                    v => Err(format!("sort() needs an array, got {}", v.type_name(p))),
                }
            }
            Builtin::Args => {
                arity(0, "args")?;
                Ok(Value::Array(Rc::new(RefCell::new(self.args.iter().map(|a| Value::str(a)).collect()))))
            }
            Builtin::Native(module, member) => {
                let mut others = Vec::new();
                let converted: Vec<SVal> = args.iter().map(|v| to_sval(p, v, &mut others)).collect();
                let result = stdlib::call(module, member, &converted)?;
                Ok(from_sval(result, &others))
            }
        }
    }
}

fn to_sval(p: &Program, v: &Value, others: &mut Vec<Value>) -> SVal {
    match v {
        Value::Null => SVal::Null,
        Value::Bool(b) => SVal::Bool(*b),
        Value::Int(i) => SVal::Int(*i),
        Value::Float(f) => SVal::Float(*f),
        Value::Str(s) => SVal::Str(s.to_string()),
        Value::Array(items) => SVal::List(items.borrow().iter().map(|x| to_sval(p, x, others)).collect()),
        Value::Dict(entries) => SVal::Map(entries.borrow().iter().map(|(k, x)| (to_sval(p, k, others), to_sval(p, x, others))).collect()),
        other => {
            let id = match others.iter().position(|o| equal(o, other)) {
                Some(i) => i,
                None => {
                    others.push(other.clone());
                    others.len() - 1
                }
            };
            SVal::Other(id, text(p, other))
        }
    }
}

fn from_sval(v: SVal, others: &[Value]) -> Value {
    match v {
        SVal::Null => Value::Null,
        SVal::Bool(b) => Value::Bool(b),
        SVal::Int(i) => Value::Int(i),
        SVal::Float(f) => Value::Float(f),
        SVal::Str(s) => Value::str(&s),
        SVal::List(items) => Value::Array(Rc::new(RefCell::new(items.into_iter().map(|x| from_sval(x, others)).collect()))),
        SVal::Map(entries) => Value::Dict(Rc::new(RefCell::new(
            entries.into_iter().map(|(k, x)| (from_sval(k, others), from_sval(x, others))).collect(),
        ))),
        SVal::Other(id, _) => others.get(id).cloned().unwrap_or(Value::Null),
    }
}

/// `(Type value)`: numbers are converted to sized types; other values must
/// already have the type.
fn convert(p: &Program, v: &Value, ty: &str, exact: bool) -> Result<Value, String> {
    if stdlib::is_numeric_type(ty) {
        let s = match v {
            Value::Int(i) => SVal::Int(*i),
            Value::Float(f) => SVal::Float(*f),
            other => return Err(format!("{} needs a number, got {}", ty, other.type_name(p))),
        };
        return Ok(match stdlib::coerce(ty, &s, exact)? {
            SVal::Int(i) => Value::Int(i),
            SVal::Float(f) => Value::Float(f),
            _ => v.clone(),
        });
    }
    let ok = match ty {
        "string" => matches!(v, Value::Str(_)),
        "bool" => matches!(v, Value::Bool(_)),
        "array" => matches!(v, Value::Array(_)),
        "dictionary" => matches!(v, Value::Dict(_)),
        name => {
            if let Some(d) = p.defs.iter().position(|d| d.name == name) {
                matches!(v, Value::Object(o) if o.def as usize == d)
            } else if let Some(e) = p.enums.iter().position(|e| e.name == name) {
                matches!(v, Value::Enum(x, _) if *x as usize == e)
            } else {
                return Err(format!("unknown type '{}'", name));
            }
        }
    };
    if ok {
        Ok(v.clone())
    } else {
        Err(format!("expected {}, got {} ({})", ty, v.type_name(p), text(p, v)))
    }
}

fn field_slot(p: &Program, o: &Object, name: &str, cache: &Cell<(u32, u32)>) -> Result<usize, String> {
    let (cdef, slot) = cache.get();
    if cdef == o.def {
        return Ok(slot as usize);
    }
    let def = &p.defs[o.def as usize];
    let slot = def.fields.iter().position(|f| f == name).ok_or_else(|| format!("{} has no field '{}'", def.name, name))?;
    cache.set((o.def, slot as u32));
    Ok(slot)
}

fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        _ => {
            let x = as_float(a)?;
            let y = as_float(b)?;
            Some(x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal))
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

fn binary(p: &Program, op: Op, l: &Value, r: &Value) -> Result<Value, String> {
    use std::cmp::Ordering::*;
    let cmp = |l: &Value, r: &Value| {
        compare(l, r).ok_or_else(|| format!("cannot compare {} with {}", l.type_name(p), r.type_name(p)))
    };
    Ok(match op {
        Op::Cat => {
            let mut s = text(p, l);
            s.push_str(&text(p, r));
            Value::Str(Rc::from(s.as_str()))
        }
        Op::Eq => Value::Bool(equal(l, r)),
        Op::Ne => Value::Bool(!equal(l, r)),
        Op::Lt => Value::Bool(cmp(l, r)? == Less),
        Op::Le => Value::Bool(cmp(l, r)? != Greater),
        Op::Gt => Value::Bool(cmp(l, r)? == Greater),
        Op::Ge => Value::Bool(cmp(l, r)? != Less),
        Op::Range => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Value::Range(*a, *b),
            _ => return Err("'..' needs two numbers".into()),
        },
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Value::Int(match op {
                Op::Add => a.wrapping_add(*b),
                Op::Sub => a.wrapping_sub(*b),
                Op::Mul => a.wrapping_mul(*b),
                Op::Div => {
                    if *b == 0 {
                        return Err("division by zero".into());
                    }
                    a.wrapping_div(*b)
                }
                _ => {
                    if *b == 0 {
                        return Err("division by zero".into());
                    }
                    a.wrapping_rem(*b)
                }
            }),
            (Value::Str(_), _) | (_, Value::Str(_)) if op == Op::Add => {
                return Err("'+' does not join text, use '.:'".into())
            }
            _ => match (as_float(l), as_float(r)) {
                (Some(a), Some(b)) => Value::Float(match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => a / b,
                    _ => a % b,
                }),
                _ => {
                    return Err(format!("operator {:?} cannot be applied to {} and {}", op, l.type_name(p), r.type_name(p)))
                }
            },
        },
        Op::BAnd | Op::BOr | Op::BXor | Op::Shl | Op::Shr => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Value::Int(match op {
                Op::BAnd => a & b,
                Op::BOr => a | b,
                Op::BXor => a ^ b,
                Op::Shl => a.wrapping_shl(*b as u32),
                _ => a.wrapping_shr(*b as u32),
            }),
            _ => return Err(format!("operator {:?} needs two numbers", op)),
        },
    })
}

fn index_of(i: i64, len: usize) -> Result<usize, String> {
    if i < 0 || i as usize >= len {
        return Err(format!("index {} is out of range (length {})", i, len));
    }
    Ok(i as usize)
}

fn range_of(a: i64, b: i64, len: usize) -> Result<(usize, usize), String> {
    if a < 0 || b < a || b as usize > len {
        return Err(format!("range {} .. {} is out of bounds (length {})", a, b, len));
    }
    Ok((a as usize, b as usize))
}

fn get_index(p: &Program, target: &Value, key: &Value) -> Result<Value, String> {
    match (target, key) {
        (Value::Array(a), Value::Int(i)) => {
            let items = a.borrow();
            Ok(items[index_of(*i, items.len())?].clone())
        }
        (Value::Array(a), Value::Range(x, y)) => {
            let items = a.borrow();
            let (s, e) = range_of(*x, *y, items.len())?;
            Ok(Value::Array(Rc::new(RefCell::new(items[s..e].to_vec()))))
        }
        (Value::Str(s), Value::Int(i)) => {
            let i = index_of(*i, char_len(s))?;
            Ok(Value::str(&char_slice(s, i, i + 1)))
        }
        (Value::Str(s), Value::Range(x, y)) => {
            let (a, b) = range_of(*x, *y, char_len(s))?;
            Ok(Value::str(&char_slice(s, a, b)))
        }
        (Value::Dict(d), key) => match d.borrow().iter().find(|(k, _)| equal(k, key)) {
            Some((_, v)) => Ok(v.clone()),
            None => Err(format!("key {} not found in dictionary", text(p, key))),
        },
        (t, k) => Err(format!("cannot index {} with {}", t.type_name(p), k.type_name(p))),
    }
}

fn set_index(p: &Program, target: &Value, key: Value, value: Value) -> Result<(), String> {
    match (target, &key) {
        (Value::Array(a), Value::Int(i)) => {
            let mut items = a.borrow_mut();
            let i = index_of(*i, items.len())?;
            items[i] = value;
            Ok(())
        }
        (Value::Dict(d), _) => {
            let mut entries = d.borrow_mut();
            match entries.iter_mut().find(|(k, _)| equal(k, &key)) {
                Some(e) => e.1 = value,
                None => entries.push((key, value)),
            }
            Ok(())
        }
        (Value::Str(_), _) => Err("strings are immutable".into()),
        (t, k) => Err(format!("cannot index {} with {}", t.type_name(p), k.type_name(p))),
    }
}

/// Loads and runs a `.clpb` file. Returns the process exit code.
pub struct VmError {
    pub message: String,
    pub frames: Vec<(String, u32, String)>,
}

fn render_vm_error(_p: &Program, e: &VmError) -> String {
    let trace: Vec<String> = e.frames.iter().map(|(name, line, file)| format!("{} ({}:{})", name, file, line)).collect();
    let (line, file) = match e.frames.first() {
        Some((_, line, file)) => (*line as usize, file.clone()),
        None => (0, String::new()),
    };
    if file.is_empty() || line == 0 {
        let mut out = format!("error: {}", e.message);
        for t in trace {
            out.push_str(&format!("\n  in {}", t));
        }
        return out + "\n";
    }
    let report = crate::diag::Report {
        severity: "error",
        message: e.message.clone(),
        file: file.clone(),
        line,
        col: 0,
        len: 0,
        help: None,
        trace,
    };
    crate::diag::render(&report, std::fs::read_to_string(&file).ok().as_deref())
}

pub fn run_file(path: &str, args: Vec<String>) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", path, e);
            return 1;
        }
    };
    let program = match load(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };
    let mut vm = Vm::new(&program, args);
    match vm.run() {
        Ok(()) => 0,
        Err(e) => {
            eprint!("{}", render_vm_error(&program, &e));
            1
        }
    }
}
