export enum class TokenKind {
    Identifier,
    Keyword,
    ReservedWord,
    DecimalIntegerLiteral,
    BinaryIntegerLiteral,
    HexIntegerLiteral,
    FloatLiteral,
    BooleanLiteral,
    NullLiteral,
    StringLiteral,
    TemplateStart,
    TemplateText,
    TemplateInterpolationStart,
    TemplateInterpolationEnd,
    TemplateEnd,
    ArithmeticOperator,
    AssignmentOperator,
    ComparisonOperator,
    LogicalOperator,
    BitwiseOperator,
    IncrementOperator,
    ConditionalOperator,
    ConcatenationOperator,
    RangeOperator,
    Ellipsis,
    MemberAccessOperator,
    ScopeResolutionOperator,
    MatchArmOperator,
    FunctionArrow,
    ManagedSubscriptionOperator,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
    Colon,
    AtSign,
    EndOfFile
}

export def Token {
    TokenKind Kind;
    << Offsets count Unicode characters; End is exclusive.
    number Start;
    number End;
    number Line;
    number Column;
}

export string GetLexeme(string source, Token token) {
    return source[token.Start .. token.End];
}
