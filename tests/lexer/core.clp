link "../../src/lexer/lexer.clp" as Lexing;
link "../../src/lexer/token.clp" as Tokens;

void ExpectToken(
    string source,
    array<Tokens.Token> tokens,
    number index,
    Tokens.TokenKind kind,
    string lexeme
) {
    if (tokens[index].Kind != kind) {
        report("unexpected token kind at index " .: index);
    }

    if (Tokens.GetLexeme(source, tokens[index]) != lexeme) {
        report("unexpected token lexeme at index " .: index);
    }
}

void ExpectLexemes(string source, array<string> expected) {
    array<Tokens.Token> tokens = Lexing.Lex(source);
    if (len(tokens) != len(expected)) {
        report("unexpected token count for source: " .: source);
    }

    number index = 0;
    while (index < len(expected)) {
        if (Tokens.GetLexeme(source, tokens[index]) != expected[index]) {
            report("unexpected token lexeme at index " .: index .: " for source: " .: source);
        }
        index += 1;
    }
}

void TestDeclarationsAndLiteralKinds() {
    string source = "number count = 42; bool ready = true; string label = \"ok\";";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    if (len(tokens) != 16) {
        report("declaration token count mismatch");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::Keyword, "number");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::Identifier, "count");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::AssignmentOperator, "=");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::DecimalIntegerLiteral, "42");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::Keyword, "bool");
    ExpectToken(source, tokens, 8, Tokens.TokenKind::BooleanLiteral, "true");
    ExpectToken(source, tokens, 10, Tokens.TokenKind::Keyword, "string");
    ExpectToken(source, tokens, 13, Tokens.TokenKind::StringLiteral, "\"ok\"");
    ExpectToken(source, tokens, 15, Tokens.TokenKind::EndOfFile, "");
}

void TestAutoContainerDeclarationAndTypedAssignmentTokens() {
    string source = "auto tabela; tabela = dictionary<string, number>({});";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    if (len(tokens) != 17) {
        report("auto container declaration token count mismatch");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::Keyword, "auto");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::Identifier, "tabela");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::Semicolon, ";");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::Identifier, "tabela");
    ExpectToken(source, tokens, 4, Tokens.TokenKind::AssignmentOperator, "=");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::Identifier, "dictionary");
    ExpectToken(source, tokens, 6, Tokens.TokenKind::ComparisonOperator, "<");
    ExpectToken(source, tokens, 7, Tokens.TokenKind::Keyword, "string");
    ExpectToken(source, tokens, 8, Tokens.TokenKind::Comma, ",");
    ExpectToken(source, tokens, 9, Tokens.TokenKind::Keyword, "number");
    ExpectToken(source, tokens, 10, Tokens.TokenKind::ComparisonOperator, ">");
    ExpectToken(source, tokens, 11, Tokens.TokenKind::LeftParen, "(");
    ExpectToken(source, tokens, 12, Tokens.TokenKind::LeftBrace, "{");
    ExpectToken(source, tokens, 13, Tokens.TokenKind::RightBrace, "}");
    ExpectToken(source, tokens, 14, Tokens.TokenKind::RightParen, ")");
    ExpectToken(source, tokens, 15, Tokens.TokenKind::Semicolon, ";");
    ExpectToken(source, tokens, 16, Tokens.TokenKind::EndOfFile, "");
}

void TestTypedCollectionSyntaxTokens() {
    ExpectLexemes(
        "dictionary<string, number> tabela = {};",
        array<string>(
            "dictionary", "<", "string", ",", "number", ">", "tabela", "=",
            "{", "}", ";", ""
        )
    );
    ExpectLexemes(
        "auto tabela = dictionary<string, number>({});",
        array<string>(
            "auto", "tabela", "=", "dictionary", "<", "string", ",", "number",
            ">", "(", "{", "}", ")", ";", ""
        )
    );
    ExpectLexemes(
        "dictionary<string, number> tabela;",
        array<string>(
            "dictionary", "<", "string", ",", "number", ">", "tabela", ";", ""
        )
    );
    ExpectLexemes(
        "auto tabela = dictionary<string, number>({Teste = 10, \"Teste2\" = 10, [\"Testesla\"] = 100});",
        array<string>(
            "auto", "tabela", "=", "dictionary", "<", "string", ",", "number",
            ">", "(", "{", "Teste", "=", "10", ",", "\"Teste2\"", "=", "10",
            ",", "[", "\"Testesla\"", "]", "=", "100", "}", ")", ";", ""
        )
    );
    ExpectLexemes(
        "auto vazia = dictionary<string, number>({});",
        array<string>(
            "auto", "vazia", "=", "dictionary", "<", "string", ",", "number",
            ">", "(", "{", "}", ")", ";", ""
        )
    );
}

void TestCommentsAndSourceLocations() {
    string source =
        "<< line comment\n"
        .: "number x = 1; <<[ block comment\n"
        .: "comment ]>> post(x);";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    if (len(tokens) != 11) {
        report("comment token count mismatch");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::Keyword, "number");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::Keyword, "post");

    if (tokens[0].Line != 2 or tokens[0].Column != 1) {
        report("line-comment source location mismatch");
    }

    if (tokens[5].Line != 3 or tokens[5].Column != 13) {
        report("block-comment source location mismatch");
    }

    if (tokens[0].Start != 16 or tokens[0].End != 22) {
        report("token source span mismatch");
    }
}

void TestNumericBasesAndFloats() {
    string source = "42 0b1010 0xFF 0xAABBCC 0xFFAABBCC 3.5 1e3 2.5e-3";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    if (len(tokens) != 9) {
        report("numeric token count mismatch");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::DecimalIntegerLiteral, "42");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::BinaryIntegerLiteral, "0b1010");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::HexIntegerLiteral, "0xFF");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::HexIntegerLiteral, "0xAABBCC");
    ExpectToken(source, tokens, 4, Tokens.TokenKind::HexIntegerLiteral, "0xFFAABBCC");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::FloatLiteral, "3.5");
    ExpectToken(source, tokens, 6, Tokens.TokenKind::FloatLiteral, "1e3");
    ExpectToken(source, tokens, 7, Tokens.TokenKind::FloatLiteral, "2.5e-3");
    ExpectToken(source, tokens, 8, Tokens.TokenKind::EndOfFile, "");
}

void TestLongestSymbolMatch() {
    string source = "... .. . ::: :: = == => -> ~>";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    if (len(tokens) != 12) {
        report("symbol token count mismatch");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::Ellipsis, "...");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::RangeOperator, "..");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::MemberAccessOperator, ".");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::ScopeResolutionOperator, "::");
    ExpectToken(source, tokens, 4, Tokens.TokenKind::Colon, ":");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::ScopeResolutionOperator, "::");
    ExpectToken(source, tokens, 6, Tokens.TokenKind::AssignmentOperator, "=");
    ExpectToken(source, tokens, 7, Tokens.TokenKind::ComparisonOperator, "==");
    ExpectToken(source, tokens, 8, Tokens.TokenKind::MatchArmOperator, "=>");
    ExpectToken(source, tokens, 9, Tokens.TokenKind::FunctionArrow, "->");
    ExpectToken(source, tokens, 10, Tokens.TokenKind::ManagedSubscriptionOperator, "~>");
    ExpectToken(source, tokens, 11, Tokens.TokenKind::EndOfFile, "");
}

void TestAdjacentOperators() {
    string source = ".....::";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    if (len(tokens) != 4) {
        report("adjacent operator token count mismatch");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::Ellipsis, "...");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::RangeOperator, "..");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::ScopeResolutionOperator, "::");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::EndOfFile, "");
}

void TestEmptyAndTriviaOnlySources() {
    array<string> sources = array<string>(
        "",
        " \t\r\n",
        "<< line comment",
        "<<[ block comment ]>>"
    );

    number index = 0;
    while (index < len(sources)) {
        array<Tokens.Token> tokens = Lexing.Lex(sources[index]);
        if (len(tokens) != 1) {
            report("empty or trivia-only source must emit only EOF");
        }

        ExpectToken(sources[index], tokens, 0, Tokens.TokenKind::EndOfFile, "");
        if (tokens[0].Start != len(sources[index]) or tokens[0].End != len(sources[index])) {
            report("EOF span must be at the end of the source");
        }
        index += 1;
    }
}

void TestKeywordAndIdentifierClassification() {
    string source =
        "thread buffer string cout endl from import namespace new static struct type using variant any array optional find sqrt typeof";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    ExpectToken(source, tokens, 0, Tokens.TokenKind::Keyword, "thread");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::Keyword, "buffer");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::Keyword, "string");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::Identifier, "cout");
    ExpectToken(source, tokens, 4, Tokens.TokenKind::Identifier, "endl");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::Identifier, "from");
    ExpectToken(source, tokens, 6, Tokens.TokenKind::Identifier, "import");
    ExpectToken(source, tokens, 7, Tokens.TokenKind::Identifier, "namespace");
    ExpectToken(source, tokens, 8, Tokens.TokenKind::Identifier, "new");
    ExpectToken(source, tokens, 9, Tokens.TokenKind::Identifier, "static");
    ExpectToken(source, tokens, 10, Tokens.TokenKind::Identifier, "struct");
    ExpectToken(source, tokens, 11, Tokens.TokenKind::Identifier, "type");
    ExpectToken(source, tokens, 12, Tokens.TokenKind::Identifier, "using");
    ExpectToken(source, tokens, 13, Tokens.TokenKind::Identifier, "variant");
    ExpectToken(source, tokens, 14, Tokens.TokenKind::Identifier, "any");
    ExpectToken(source, tokens, 15, Tokens.TokenKind::Identifier, "array");
    ExpectToken(source, tokens, 16, Tokens.TokenKind::Identifier, "optional");
    ExpectToken(source, tokens, 17, Tokens.TokenKind::Identifier, "find");
    ExpectToken(source, tokens, 18, Tokens.TokenKind::Identifier, "sqrt");
    ExpectToken(source, tokens, 19, Tokens.TokenKind::Keyword, "typeof");
}

void TestOperatorCategories() {
    string source = "+ - * / % = += -= *= /= %= == != < <= > >= ! & | ^ ~ ? . : @ :: .: .. ... => -> ~> ++ --";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    ExpectToken(source, tokens, 0, Tokens.TokenKind::ArithmeticOperator, "+");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::AssignmentOperator, "=");
    ExpectToken(source, tokens, 6, Tokens.TokenKind::AssignmentOperator, "+=");
    ExpectToken(source, tokens, 11, Tokens.TokenKind::ComparisonOperator, "==");
    ExpectToken(source, tokens, 13, Tokens.TokenKind::ComparisonOperator, "<");
    ExpectToken(source, tokens, 14, Tokens.TokenKind::ComparisonOperator, "<=");
    ExpectToken(source, tokens, 17, Tokens.TokenKind::LogicalOperator, "!");
    ExpectToken(source, tokens, 18, Tokens.TokenKind::BitwiseOperator, "&");
    ExpectToken(source, tokens, 22, Tokens.TokenKind::ConditionalOperator, "?");
    ExpectToken(source, tokens, 23, Tokens.TokenKind::MemberAccessOperator, ".");
    ExpectToken(source, tokens, 25, Tokens.TokenKind::AtSign, "@");
    ExpectToken(source, tokens, 26, Tokens.TokenKind::ScopeResolutionOperator, "::");
    ExpectToken(source, tokens, 27, Tokens.TokenKind::ConcatenationOperator, ".:");
    ExpectToken(source, tokens, 28, Tokens.TokenKind::RangeOperator, "..");
    ExpectToken(source, tokens, 29, Tokens.TokenKind::Ellipsis, "...");
    ExpectToken(source, tokens, 30, Tokens.TokenKind::MatchArmOperator, "=>");
    ExpectToken(source, tokens, 31, Tokens.TokenKind::FunctionArrow, "->");
    ExpectToken(source, tokens, 32, Tokens.TokenKind::ManagedSubscriptionOperator, "~>");
    ExpectToken(source, tokens, 33, Tokens.TokenKind::IncrementOperator, "++");
    ExpectToken(source, tokens, 34, Tokens.TokenKind::IncrementOperator, "--");
    ExpectToken(source, tokens, 35, Tokens.TokenKind::EndOfFile, "");
}

void TestTextOperatorsAndNamespacedCalls() {
    string source = "and or not in string.find(\"x\", \"y\") math.sqrt(4) shl shr";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    ExpectToken(source, tokens, 0, Tokens.TokenKind::LogicalOperator, "and");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::LogicalOperator, "or");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::LogicalOperator, "not");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::Keyword, "in");
    ExpectToken(source, tokens, 4, Tokens.TokenKind::Keyword, "string");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::MemberAccessOperator, ".");
    ExpectToken(source, tokens, 6, Tokens.TokenKind::Identifier, "find");
    ExpectToken(source, tokens, 12, Tokens.TokenKind::Identifier, "math");
    ExpectToken(source, tokens, 14, Tokens.TokenKind::Identifier, "sqrt");
    ExpectToken(source, tokens, 18, Tokens.TokenKind::BitwiseOperator, "shl");
    ExpectToken(source, tokens, 19, Tokens.TokenKind::BitwiseOperator, "shr");
}

void TestTemplateInterpolation() {
    string source = "post(`hello ${name}`);";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    if (len(tokens) != 11) {
        report("template token count mismatch");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::Keyword, "post");
    ExpectToken(source, tokens, 2, Tokens.TokenKind::TemplateStart, "`");
    ExpectToken(source, tokens, 3, Tokens.TokenKind::TemplateText, "hello ");
    ExpectToken(source, tokens, 4, Tokens.TokenKind::TemplateInterpolationStart, "${");
    ExpectToken(source, tokens, 5, Tokens.TokenKind::Identifier, "name");
    ExpectToken(source, tokens, 6, Tokens.TokenKind::TemplateInterpolationEnd, "}");
    ExpectToken(source, tokens, 7, Tokens.TokenKind::TemplateEnd, "`");
    ExpectToken(source, tokens, 10, Tokens.TokenKind::EndOfFile, "");
}

void TestNestedAndEscapedTemplates() {
    string nestedSource = "`A ${`B ${name}`} C`";
    array<Tokens.Token> nested = Lexing.Lex(nestedSource);

    ExpectToken(nestedSource, nested, 0, Tokens.TokenKind::TemplateStart, "`");
    ExpectToken(nestedSource, nested, 2, Tokens.TokenKind::TemplateInterpolationStart, "${");
    ExpectToken(nestedSource, nested, 3, Tokens.TokenKind::TemplateStart, "`");
    ExpectToken(nestedSource, nested, 5, Tokens.TokenKind::TemplateInterpolationStart, "${");
    ExpectToken(nestedSource, nested, 7, Tokens.TokenKind::TemplateInterpolationEnd, "}");
    ExpectToken(nestedSource, nested, 8, Tokens.TokenKind::TemplateEnd, "`");
    ExpectToken(nestedSource, nested, 9, Tokens.TokenKind::TemplateInterpolationEnd, "}");
    ExpectToken(nestedSource, nested, 11, Tokens.TokenKind::TemplateEnd, "`");
    ExpectToken(nestedSource, nested, 12, Tokens.TokenKind::EndOfFile, "");

    string escapedSource = "`\\${value}`";
    array<Tokens.Token> escaped = Lexing.Lex(escapedSource);
    if (len(escaped) != 4) {
        report("escaped template interpolation token count mismatch");
    }
    ExpectToken(escapedSource, escaped, 0, Tokens.TokenKind::TemplateStart, "`");
    ExpectToken(escapedSource, escaped, 1, Tokens.TokenKind::TemplateText, "\\${value}");
    ExpectToken(escapedSource, escaped, 2, Tokens.TokenKind::TemplateEnd, "`");
}

void TestDeepTemplateNesting() {
    string source = "`value`";
    number depth = 0;
    while (depth < 256) {
        source = "`${" .: source .: "}`";
        depth += 1;
    }

    array<Tokens.Token> tokens = Lexing.Lex(source);
    if (len(tokens) != 1028) {
        report("deeply nested templates produced an unexpected token count");
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::TemplateStart, "`");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::TemplateInterpolationStart, "${");
    ExpectToken(source, tokens, 1027, Tokens.TokenKind::EndOfFile, "");
}

void TestEscapedQuotedStrings() {
    string doubleQuotedSource = "\"quoted \\\" word\"";
    array<Tokens.Token> doubleQuoted = Lexing.Lex(doubleQuotedSource);
    ExpectToken(
        doubleQuotedSource,
        doubleQuoted,
        0,
        Tokens.TokenKind::StringLiteral,
        "\"quoted \\\" word\""
    );

    string singleQuotedSource = "'it\\'s'";
    array<Tokens.Token> singleQuoted = Lexing.Lex(singleQuotedSource);
    ExpectToken(
        singleQuotedSource,
        singleQuoted,
        0,
        Tokens.TokenKind::StringLiteral,
        "'it\\'s'"
    );
}

void TestMalformedInputReportsErrors() {
    array<string> invalidSources = array<string>(
        "<<[ unterminated",
        "\"unterminated",
        "\"trailing\\",
        "\"bad\\qescape\"",
        "0b2",
        "0b12",
        "0b1_2",
        "0b102",
        "0b10name",
        "0x",
        "0xG",
        "0x_FF",
        "10_",
        "1__0",
        "1e+",
        "1e3suffix",
        "1name",
        "number x = 1; <<[ unterminated",
        "`unterminated",
        "`value ${name`",
        "\"line\nbreak\"",
        "$",
        "number x = 1;$"
    );

    number index = 0;
    while (index < len(invalidSources)) {
        bool caughtError = false;
        try {
            Lexing.Lex(invalidSources[index]);
        } catch (error) {
            caughtError = true;
        }

        if (not caughtError) {
            report("invalid source was not rejected at test index " .: index);
        }
        index += 1;
    }
}

void TestUnicodeSourceOffsets() {
    string source = "\"é\" name";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    ExpectToken(source, tokens, 0, Tokens.TokenKind::StringLiteral, "\"é\"");
    ExpectToken(source, tokens, 1, Tokens.TokenKind::Identifier, "name");

    if (tokens[1].Start != 4 or tokens[1].End != 8) {
        report("source offsets must count Unicode characters");
    }

    if (tokens[1].Line != 1 or tokens[1].Column != 5) {
        report("column must count Unicode characters");
    }
}

void TestCRLFPositions() {
    string source = "first\r\nsecond";
    array<Tokens.Token> tokens = Lexing.Lex(source);

    ExpectToken(source, tokens, 1, Tokens.TokenKind::Identifier, "second");
    if (tokens[1].Start != 7 or tokens[1].Line != 2 or tokens[1].Column != 1) {
        report("CRLF must advance one line and preserve character offsets");
    }
}

void TestDocCommentTokens() {
    string source = "<<{ !desc = \"x\" }>>\n<< plain\nnumber a"
    array<Tokens.Token> tokens = Lexing.Lex(source)

    if (len(tokens) != 9) {
        report("doc comment token count mismatch: " .: len(tokens))
        return
    }

    ExpectToken(source, tokens, 0, Tokens.TokenKind::DocCommentStart, "<<{")
    ExpectToken(source, tokens, 1, Tokens.TokenKind::LogicalOperator, "!")
    ExpectToken(source, tokens, 2, Tokens.TokenKind::Identifier, "desc")
    ExpectToken(source, tokens, 4, Tokens.TokenKind::StringLiteral, "\"x\"")
    ExpectToken(source, tokens, 5, Tokens.TokenKind::DocCommentEnd, "}>>")
    ExpectToken(source, tokens, 6, Tokens.TokenKind::Keyword, "number")
}

TestDeclarationsAndLiteralKinds();
TestAutoContainerDeclarationAndTypedAssignmentTokens();
TestTypedCollectionSyntaxTokens();
TestCommentsAndSourceLocations();
TestNumericBasesAndFloats();
TestLongestSymbolMatch();
TestAdjacentOperators();
TestEmptyAndTriviaOnlySources();
TestKeywordAndIdentifierClassification();
TestOperatorCategories();
TestTextOperatorsAndNamespacedCalls();
TestTemplateInterpolation();
TestNestedAndEscapedTemplates();
TestDeepTemplateNesting();
TestEscapedQuotedStrings();
TestMalformedInputReportsErrors();
TestUnicodeSourceOffsets();
TestCRLFPositions();
TestDocCommentTokens()
post("lexer core tests passed");
