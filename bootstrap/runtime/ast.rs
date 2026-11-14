//! Syntax tree used by the stage-0 interpreter.

use std::rc::Rc;

/// 1-based source position of a name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct TypeName {
    /// Qualified base name, e.g. `number`, `Ast.Node`, `array`.
    pub base: String,
    pub args: Vec<TypeName>,
}

impl TypeName {
    pub fn render(&self) -> String {
        if self.args.is_empty() {
            return self.base.clone();
        }
        let args: Vec<String> = self.args.iter().map(TypeName::render).collect();
        format!("{}<{}>", self.base, args.join(", "))
    }
}

#[derive(Debug, Clone)]
pub enum DictKey {
    /// Bare identifier key: `{ Name = 1 }` means the text "Name".
    Name(String),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Null,
    Ident(String),
    This,
    Member(Box<Expr>, String),
    Scope(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Unary(String, Box<Expr>),
    Postfix(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Assign(String, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    New(Box<Expr>, Vec<(String, Expr)>),
    ArrayLit(Vec<Expr>),
    DictLit(Vec<(DictKey, Expr)>),
    /// `array<T>(...)` / `dictionary<K, V>(...)`.
    TypedCollection(TypeName, Vec<Expr>),
    /// `(Type value)`: the value must have exactly this type.
    Typed(TypeName, Box<Expr>),
    /// `func (Type a) -> Type { ... }`: captures outer variables by value.
    Lambda(Rc<FnDecl>),
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Var { name: String, ty: Option<TypeName>, init: Option<Expr>, pos: Pos },
    Expr(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    DoWhile(Vec<Stmt>, Expr),
    ForIn { name: String, ty: Option<TypeName>, iterable: Expr, body: Vec<Stmt>, pos: Pos },
    Guard(Expr, Vec<Stmt>),
    Try { body: Vec<Stmt>, binding: Option<String>, handler: Vec<Stmt> },
    Break,
    Continue,
    Return(Option<Expr>),
    Block(Vec<Stmt>),
    /// `match (value) { pattern, pattern => body ... _ => body }`;
    /// a `None` pattern is `_`.
    Match(Expr, Vec<MatchArm>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub patterns: Vec<Option<Expr>>,
    pub body: Vec<Stmt>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub line: usize,
}

#[derive(Debug)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<String>,
    pub param_types: Vec<TypeName>,
    pub ret: TypeName,
    pub body: Vec<Stmt>,
    pub line: usize,
    pub pos: Pos,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeName,
    pub default: Option<Expr>,
    pub pos: Pos,
}

#[derive(Debug)]
pub struct DefDecl {
    pub name: String,
    /// `def Mage:Entity`: the base def.
    pub base: Option<TypeName>,
    /// `def Box<T>`: type parameter names.
    pub type_params: Vec<String>,
    pub pos: Pos,
    pub end_line: usize,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<Rc<FnDecl>>,
}

#[derive(Debug)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub pos: Pos,
    pub variant_pos: Vec<Pos>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Link { path: String, alias: Option<String>, line: usize, pos: Pos },
    Function { decl: Rc<FnDecl>, exported: bool },
    Def { decl: Rc<DefDecl>, exported: bool },
    /// `def X = <type expression>`; has no run-time effect.
    TypeAlias { name: String, exported: bool, pos: Pos },
    Enum { decl: Rc<EnumDecl>, exported: bool },
    Global { stmt: Stmt, exported: bool },
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub module_name: String,
    pub items: Vec<Item>,
}
