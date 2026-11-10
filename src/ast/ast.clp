export enum class NodeKind {
    Module,
    LinkDeclaration,
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
    ConditionalExpression
}

export def Node {
    NodeKind Kind;
    string Name;
    string TypeName;
    string Value;
    bool IsExported;
    bool IsAuto;
    number StartToken;
    number EndToken;
    array<Node> Children;
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
