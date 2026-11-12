link "./token.clp" as TokenModule;

const array<string> DIGITS = array<string>(
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"
);
const array<string> HEX_DIGITS = array<string>(
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
    "a", "b", "c", "d", "e", "f",
    "A", "B", "C", "D", "E", "F"
);
const array<string> LETTERS = array<string>(
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
    "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
    "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
    "_"
);
const array<string> KEYWORDS = array<string>(
    "Vector2", "Vector3", "Vector4", "abstract", "and", "as", "async",
    "atomic", "auto", "await", "bool", "break", "buffer", "case", "catch",
    "class", "const", "constexpr", "continue", "decltype", "def", "default",
    "do", "else", "enum", "export", "extern", "false", "final", "float",
    "for", "func", "guard", "if", "in", "join", "link", "match", "move",
    "not", "null", "number", "observable", "operator", "or", "override",
    "parallel", "pcall", "post", "pragma", "private", "public", "report",
    "return", "shl", "shr", "signal", "spawn", "static_cast", "string",
    "switch", "task", "template", "thread", "true", "try", "typename",
    "void", "warn", "where", "while", "typeof"
);
def SymbolRule {
    string Text;
    TokenModule.TokenKind Kind;
}

def TemplateFrame {
    number StartPosition;
    number StartLine;
    number StartColumn;
    bool InInterpolation;
    number BraceDepth;
}

const array<SymbolRule> SYMBOL_RULES = array<SymbolRule>(
    SymbolRule { Text = "...", Kind = TokenModule.TokenKind::Ellipsis },
    SymbolRule { Text = "::", Kind = TokenModule.TokenKind::ScopeResolutionOperator },
    SymbolRule { Text = ".:", Kind = TokenModule.TokenKind::ConcatenationOperator },
    SymbolRule { Text = "..", Kind = TokenModule.TokenKind::RangeOperator },
    SymbolRule { Text = "=>", Kind = TokenModule.TokenKind::MatchArmOperator },
    SymbolRule { Text = "->", Kind = TokenModule.TokenKind::FunctionArrow },
    SymbolRule { Text = "~>", Kind = TokenModule.TokenKind::ManagedSubscriptionOperator },
    SymbolRule { Text = "==", Kind = TokenModule.TokenKind::ComparisonOperator },
    SymbolRule { Text = "!=", Kind = TokenModule.TokenKind::ComparisonOperator },
    SymbolRule { Text = "<=", Kind = TokenModule.TokenKind::ComparisonOperator },
    SymbolRule { Text = ">=", Kind = TokenModule.TokenKind::ComparisonOperator },
    SymbolRule { Text = "+=", Kind = TokenModule.TokenKind::AssignmentOperator },
    SymbolRule { Text = "-=", Kind = TokenModule.TokenKind::AssignmentOperator },
    SymbolRule { Text = "*=", Kind = TokenModule.TokenKind::AssignmentOperator },
    SymbolRule { Text = "/=", Kind = TokenModule.TokenKind::AssignmentOperator },
    SymbolRule { Text = "%=", Kind = TokenModule.TokenKind::AssignmentOperator },
    SymbolRule { Text = "++", Kind = TokenModule.TokenKind::IncrementOperator },
    SymbolRule { Text = "--", Kind = TokenModule.TokenKind::IncrementOperator },
    SymbolRule { Text = "+", Kind = TokenModule.TokenKind::ArithmeticOperator },
    SymbolRule { Text = "-", Kind = TokenModule.TokenKind::ArithmeticOperator },
    SymbolRule { Text = "*", Kind = TokenModule.TokenKind::ArithmeticOperator },
    SymbolRule { Text = "/", Kind = TokenModule.TokenKind::ArithmeticOperator },
    SymbolRule { Text = "%", Kind = TokenModule.TokenKind::ArithmeticOperator },
    SymbolRule { Text = "=", Kind = TokenModule.TokenKind::AssignmentOperator },
    SymbolRule { Text = "<", Kind = TokenModule.TokenKind::ComparisonOperator },
    SymbolRule { Text = ">", Kind = TokenModule.TokenKind::ComparisonOperator },
    SymbolRule { Text = "!", Kind = TokenModule.TokenKind::LogicalOperator },
    SymbolRule { Text = "&", Kind = TokenModule.TokenKind::BitwiseOperator },
    SymbolRule { Text = "|", Kind = TokenModule.TokenKind::BitwiseOperator },
    SymbolRule { Text = "^", Kind = TokenModule.TokenKind::BitwiseOperator },
    SymbolRule { Text = "~", Kind = TokenModule.TokenKind::BitwiseOperator },
    SymbolRule { Text = "?", Kind = TokenModule.TokenKind::ConditionalOperator },
    SymbolRule { Text = ".", Kind = TokenModule.TokenKind::MemberAccessOperator },
    SymbolRule { Text = ":", Kind = TokenModule.TokenKind::Colon },
    SymbolRule { Text = "@", Kind = TokenModule.TokenKind::AtSign },
    SymbolRule { Text = "(", Kind = TokenModule.TokenKind::LeftParen },
    SymbolRule { Text = ")", Kind = TokenModule.TokenKind::RightParen },
    SymbolRule { Text = "[", Kind = TokenModule.TokenKind::LeftBracket },
    SymbolRule { Text = "]", Kind = TokenModule.TokenKind::RightBracket },
    SymbolRule { Text = "{", Kind = TokenModule.TokenKind::LeftBrace },
    SymbolRule { Text = "}", Kind = TokenModule.TokenKind::RightBrace },
    SymbolRule { Text = ",", Kind = TokenModule.TokenKind::Comma },
    SymbolRule { Text = ";", Kind = TokenModule.TokenKind::Semicolon }
);

export def Lexer {
    string Source;
    number SourceLength;
    array<TemplateFrame> TemplateFrames;
    number TemplateDepth;
    array<TokenModule.Token> Tokens;
    number Position;
    number Line;
    number Column;

    void Advance() {
        string character = @this.Source[@this.Position];

        if (character == "\n") {
            @this.Line += 1;
            @this.Column = 1;
        } else if (character == "\r") {
            if (@this.Position + 1 >= @this.SourceLength or @this.Source[@this.Position + 1] != "\n") {
                @this.Line += 1;
                @this.Column = 1;
            }
        } else {
            @this.Column += 1;
        }

        @this.Position += 1;
    }

    bool IsDigit(string character) {
        return find(DIGITS, character) >= 0;
    }

    bool IsHexDigit(string character) {
        return find(HEX_DIGITS, character) >= 0;
    }

    bool IsLetter(string character) {
        return find(LETTERS, character) >= 0;
    }

    bool IsIdentifierStart(string character) {
        return @this.IsLetter(character);
    }

    bool IsIdentifierPart(string character) {
        return @this.IsLetter(character) or @this.IsDigit(character);
    }

    bool Match(string text) {
        number end = @this.Position + len(text);

        return end <= @this.SourceLength
            and @this.Source[@this.Position .. end] == text;
    }

    void Emit(
        TokenModule.TokenKind kind,
        number startPosition,
        number startLine,
        number startColumn
    ) {
        push(@this.Tokens, TokenModule.Token {
            Kind = kind,
            Start = startPosition,
            End = @this.Position,
            Line = startLine,
            Column = startColumn
        });
    }

    void SkipTrivia() {
        while (@this.Position < @this.SourceLength) {
            string character = @this.Source[@this.Position];

            if (character == " " or character == "\t" or character == "\n" or character == "\r") {
                @this.Advance();
            } else if (@this.Match("<<[")) {
                number commentLine = @this.Line;
                number commentColumn = @this.Column;

                @this.Advance();
                @this.Advance();
                @this.Advance();

                while (@this.Position < @this.SourceLength and not @this.Match("]>>")) {
                    @this.Advance();
                }

                if (not @this.Match("]>>")) {
                    report("unterminated block comment at " .: commentLine .: ":" .: commentColumn);
                    return;
                }

                @this.Advance();
                @this.Advance();
                @this.Advance();
            } else if (@this.Match("<<")) {
                while (
                    @this.Position < @this.SourceLength
                    and @this.Source[@this.Position] != "\n"
                    and @this.Source[@this.Position] != "\r"
                ) {
                    @this.Advance();
                }
            } else {
                return;
            }
        }
    }

    bool IsKeyword(string text) {
        return find(KEYWORDS, text) >= 0;
    }

    void ScanIdentifier(number startPosition, number startLine, number startColumn) {
        while (
            @this.Position < @this.SourceLength
            and @this.IsIdentifierPart(@this.Source[@this.Position])
        ) {
            @this.Advance();
        }

        string text = @this.Source[startPosition .. @this.Position];
        TokenModule.TokenKind kind = TokenModule.TokenKind::Identifier;

        if (text == "true" or text == "false") {
            kind = TokenModule.TokenKind::BooleanLiteral;
        } else if (text == "null") {
            kind = TokenModule.TokenKind::NullLiteral;
        } else if (text == "and" or text == "or" or text == "not") {
            kind = TokenModule.TokenKind::LogicalOperator;
        } else if (text == "shl" or text == "shr") {
            kind = TokenModule.TokenKind::BitwiseOperator;
        } else if (@this.IsKeyword(text)) {
            kind = TokenModule.TokenKind::Keyword;
        }

        @this.Emit(kind, startPosition, startLine, startColumn);
    }

    void ScanDigits(bool hexadecimal, number startLine, number startColumn) {
        bool foundDigit = false;
        bool previousWasDigit = false;

        while (@this.Position < @this.SourceLength) {
            string character = @this.Source[@this.Position];
            bool isDigit = @this.IsDigit(character)
                or (hexadecimal and @this.IsHexDigit(character));

            if (isDigit) {
                foundDigit = true;
                previousWasDigit = true;
                @this.Advance();
            } else if (character == "_") {
                if (not previousWasDigit) {
                    report("invalid digit separator at " .: @this.Line .: ":" .: @this.Column);
                    return;
                }

                previousWasDigit = false;
                @this.Advance();
            } else {
                break;
            }
        }

        if (not foundDigit or not previousWasDigit) {
            report("invalid numeric literal at " .: startLine .: ":" .: startColumn);
        }
    }

    void ScanNumber(number startPosition, number startLine, number startColumn) {
        bool isFloat = false;
        bool isBinary = false;
        bool isHexadecimal = false;

        if (
            @this.Source[@this.Position] == "0"
            and @this.Position + 1 < @this.SourceLength
            and (
                @this.Source[@this.Position + 1] == "x"
                or @this.Source[@this.Position + 1] == "X"
            )
        ) {
            isHexadecimal = true;
            @this.Advance();
            @this.Advance();
            @this.ScanDigits(true, startLine, startColumn);
        } else if (
            @this.Source[@this.Position] == "0"
            and @this.Position + 1 < @this.SourceLength
            and (
                @this.Source[@this.Position + 1] == "b"
                or @this.Source[@this.Position + 1] == "B"
            )
        ) {
            isBinary = true;
            @this.Advance();
            @this.Advance();

            bool foundDigit = false;
            bool previousWasDigit = false;
            while (
                @this.Position < @this.SourceLength
                and (
                    @this.Source[@this.Position] == "0"
                    or @this.Source[@this.Position] == "1"
                    or @this.Source[@this.Position] == "_"
                )
            ) {
                string character = @this.Source[@this.Position];
                if (character == "_") {
                    if (not previousWasDigit) {
                        report("invalid digit separator at " .: @this.Line .: ":" .: @this.Column);
                        return;
                    }
                    previousWasDigit = false;
                } else {
                    foundDigit = true;
                    previousWasDigit = true;
                }
                @this.Advance();
            }

            if (not foundDigit or not previousWasDigit) {
                report("invalid numeric literal at " .: startLine .: ":" .: startColumn);
            }
        } else {
            @this.ScanDigits(false, startLine, startColumn);

            if (
                @this.Position < @this.SourceLength
                and @this.Source[@this.Position] == "."
                and not @this.Match("..")
                and @this.Position + 1 < @this.SourceLength
                and @this.IsDigit(@this.Source[@this.Position + 1])
            ) {
                isFloat = true;
                @this.Advance();
                @this.ScanDigits(false, startLine, startColumn);
            }

            if (
                @this.Position < @this.SourceLength
                and (
                    @this.Source[@this.Position] == "e"
                    or @this.Source[@this.Position] == "E"
                )
            ) {
                isFloat = true;
                @this.Advance();
                if (
                    @this.Position < @this.SourceLength
                    and (
                        @this.Source[@this.Position] == "+"
                        or @this.Source[@this.Position] == "-"
                    )
                ) {
                    @this.Advance();
                }
                @this.ScanDigits(false, startLine, startColumn);
            }
        }

        if (
            @this.Position < @this.SourceLength
            and @this.IsIdentifierPart(@this.Source[@this.Position])
        ) {
            report("invalid character after numeric literal at " .: @this.Line .: ":" .: @this.Column);
            return;
        }

        if (isFloat) {
            @this.Emit(TokenModule.TokenKind::FloatLiteral, startPosition, startLine, startColumn);
        } else if (isBinary) {
            @this.Emit(TokenModule.TokenKind::BinaryIntegerLiteral, startPosition, startLine, startColumn);
        } else if (isHexadecimal) {
            @this.Emit(TokenModule.TokenKind::HexIntegerLiteral, startPosition, startLine, startColumn);
        } else {
            @this.Emit(TokenModule.TokenKind::DecimalIntegerLiteral, startPosition, startLine, startColumn);
        }
    }

    void ScanQuotedString(
        string quote,
        number startPosition,
        number startLine,
        number startColumn
    ) {
        @this.Advance();

        while (@this.Position < @this.SourceLength and @this.Source[@this.Position] != quote) {
            string character = @this.Source[@this.Position];

            if (character == "\n" or character == "\r") {
                report("newline in quoted string at " .: @this.Line .: ":" .: @this.Column);
                return;
            }

            if (character == "\\") {
                @this.Advance();
                if (@this.Position >= @this.SourceLength) {
                    report("unterminated escape sequence at " .: @this.Line .: ":" .: @this.Column);
                    return;
                }

                character = @this.Source[@this.Position];
                if (
                    character != "n"
                    and character != "t"
                    and character != "\\"
                    and character != "\""
                    and character != "'"
                ) {
                    report("invalid escape sequence at " .: @this.Line .: ":" .: @this.Column);
                    return;
                }
            }

            @this.Advance();
        }

        if (@this.Position >= @this.SourceLength) {
            report("unterminated string at " .: startLine .: ":" .: startColumn);
            return;
        }

        @this.Advance();
        @this.Emit(TokenModule.TokenKind::StringLiteral, startPosition, startLine, startColumn);
    }

    void BeginTemplate(
        number startPosition,
        number startLine,
        number startColumn
    ) {
        TemplateFrame frame = TemplateFrame {
            StartPosition = startPosition,
            StartLine = startLine,
            StartColumn = startColumn,
            InInterpolation = false,
            BraceDepth = 0
        };

        if (@this.TemplateDepth < len(@this.TemplateFrames)) {
            @this.TemplateFrames[@this.TemplateDepth] = frame;
        } else {
            push(@this.TemplateFrames, frame);
        }

        @this.TemplateDepth += 1;
        @this.Advance();
        @this.Emit(TokenModule.TokenKind::TemplateStart, startPosition, startLine, startColumn);
    }

    void ScanTemplateText() {
        number frameIndex = @this.TemplateDepth - 1;
        number textStart = @this.Position;
        number textLine = @this.Line;
        number textColumn = @this.Column;

        while (@this.Position < @this.SourceLength) {
            if (@this.Source[@this.Position] == "`" or @this.Match("${")) {
                break;
            }

            if (
                @this.Source[@this.Position] == "\\"
                and @this.Position + 2 < @this.SourceLength
                and @this.Source[@this.Position + 1] == "$"
                and @this.Source[@this.Position + 2] == "{"
            ) {
                @this.Advance();
                @this.Advance();
                @this.Advance();
            } else {
                @this.Advance();
            }
        }

        if (@this.Position > textStart) {
            @this.Emit(
                TokenModule.TokenKind::TemplateText,
                textStart,
                textLine,
                textColumn
            );
        }

        if (@this.Position >= @this.SourceLength) {
            return;
        }

        if (@this.Match("${")) {
            number interpolationStart = @this.Position;
            number interpolationLine = @this.Line;
            number interpolationColumn = @this.Column;
            @this.Advance();
            @this.Advance();
            @this.Emit(
                TokenModule.TokenKind::TemplateInterpolationStart,
                interpolationStart,
                interpolationLine,
                interpolationColumn
            );
            TemplateFrame frame = @this.TemplateFrames[frameIndex];
            frame.InInterpolation = true;
            frame.BraceDepth = 0;
            @this.TemplateFrames[frameIndex] = frame;
        } else {
            number endPosition = @this.Position;
            number endLine = @this.Line;
            number endColumn = @this.Column;
            @this.Advance();
            @this.Emit(
                TokenModule.TokenKind::TemplateEnd,
                endPosition,
                endLine,
                endColumn
            );
            @this.TemplateDepth -= 1;
        }
    }

    void ScanSymbol(number startPosition, number startLine, number startColumn) {
        number index = 0;
        while (index < len(SYMBOL_RULES)) {
            SymbolRule rule = SYMBOL_RULES[index];
            if (@this.Match(rule.Text)) {
                while (@this.Position < startPosition + len(rule.Text)) {
                    @this.Advance();
                }
                @this.Emit(rule.Kind, startPosition, startLine, startColumn);
                return;
            }
            index += 1;
        }

        report("unexpected character at " .: startLine .: ":" .: startColumn);
    }

    void ScanToken() {
        number startPosition = @this.Position;
        number startLine = @this.Line;
        number startColumn = @this.Column;
        string character = @this.Source[@this.Position];

        if (@this.IsIdentifierStart(character)) {
            @this.ScanIdentifier(startPosition, startLine, startColumn);
        } else if (@this.IsDigit(character)) {
            @this.ScanNumber(startPosition, startLine, startColumn);
        } else if (character == "\"" or character == "'") {
            @this.ScanQuotedString(character, startPosition, startLine, startColumn);
        } else if (character == "`") {
            @this.BeginTemplate(startPosition, startLine, startColumn);
        } else {
            @this.ScanSymbol(startPosition, startLine, startColumn);
        }
    }

    array<TokenModule.Token> ScanTokens() {
        while (@this.Position < @this.SourceLength) {
            if (
                @this.TemplateDepth > 0
                and not @this.TemplateFrames[@this.TemplateDepth - 1].InInterpolation
            ) {
                @this.ScanTemplateText();
            } else {
                @this.SkipTrivia();
                if (@this.Position >= @this.SourceLength) {
                    break;
                }

                if (@this.TemplateDepth > 0) {
                    number frameIndex = @this.TemplateDepth - 1;
                    string character = @this.Source[@this.Position];
                    TemplateFrame frame = @this.TemplateFrames[frameIndex];

                    if (character == "}" and frame.BraceDepth == 0) {
                        number startPosition = @this.Position;
                        number startLine = @this.Line;
                        number startColumn = @this.Column;
                        @this.Advance();
                        @this.Emit(
                            TokenModule.TokenKind::TemplateInterpolationEnd,
                            startPosition,
                            startLine,
                            startColumn
                        );
                        frame.InInterpolation = false;
                        @this.TemplateFrames[frameIndex] = frame;
                        continue;
                    }

                    if (character == "{") {
                        frame.BraceDepth += 1;
                    } else if (character == "}") {
                        frame.BraceDepth -= 1;
                    }
                    @this.TemplateFrames[frameIndex] = frame;
                }

                if (@this.Source[@this.Position] == "`") {
                    @this.BeginTemplate(@this.Position, @this.Line, @this.Column);
                } else {
                    @this.ScanToken();
                }
            }
        }

        if (@this.TemplateDepth > 0) {
            TemplateFrame frame = @this.TemplateFrames[@this.TemplateDepth - 1];
            if (frame.InInterpolation) {
                report("unterminated template interpolation at " .: @this.Line .: ":" .: @this.Column);
            } else {
                report("unterminated template at " .: frame.StartLine .: ":" .: frame.StartColumn
                );
            }
            return @this.Tokens;
        }

        @this.Emit(
            TokenModule.TokenKind::EndOfFile,
            @this.Position,
            @this.Line,
            @this.Column
        );
        return @this.Tokens;
    }
}

export array<TokenModule.Token> Lex(string source) {
    Lexer lexer = Lexer {
        Source = source,
        SourceLength = len(source),
        TemplateFrames = array<TemplateFrame>(),
        TemplateDepth = 0,
        Tokens = array<TokenModule.Token>(),
        Position = 0,
        Line = 1,
        Column = 1
    };

    return lexer.ScanTokens();
}
