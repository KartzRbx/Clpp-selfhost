export enum class NodeKind {
    Module,
    LinkDeclaration,
    TypeDeclaration,
    DefDeclaration,
    FieldDeclaration,
    EnumDeclaration,
    EnumMember,
    TypeReference,
    TypeIntersection,
    ObjectType,
    TypeField,
    TypeofType,
    VariableDeclaration,
    FunctionDeclaration,
    Parameter,
    Block,
    Assignment,
    ReturnStatement,
    ExpressionStatement,
    IdentifierExpression,
    LiteralExpression,
    UnaryExpression,
    BinaryExpression,
    CallExpression,
    MemberExpression,
    ArrayLiteral,
    DictionaryLiteral,
    TypedCollectionExpression,
    TypedExpression,
    ConditionalExpression,
    ThisExpression,
    ScopeExpression,
    ObjectConstruction,
    FieldInitializer,
    IfStatement,
    WhileStatement,
    DoWhileStatement,
    ForInStatement,
    GuardStatement,
    TryStatement,
    BreakStatement,
    ContinueStatement
}

export def DocTag {
    string Name;
    string Value;
}

export def Node {
    NodeKind Kind;
    string Name;
    string TypeName;
    string Value;
    bool IsExported;
    bool IsAuto;
    bool IsConst;
    number StartToken;
    number EndToken;
    array<Node> Children;
    << Tags from a `<<{ !tag = "..." }>>` doc comment written above the declaration.
    array<DocTag> Docs;
}

export def NamingRule {
    string Target;
    string Convention;
}

export def Module {
    string Name;
    array<NamingRule> NamingRules;
    array<Node> Declarations;
}
