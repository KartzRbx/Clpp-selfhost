link "../ast/ast.clp" as Ast;
link "../lexer/token.clp" as TokenModule;

export def Parser {
    string Source;
    array<TokenModule.Token> Tokens;
    number Current;

    bool IsAtEnd() {
        return @this.Tokens[@this.Current].Kind == TokenModule.TokenKind::EndOfFile;
    }

    string Text(number index) {
        return TokenModule.GetLexeme(@this.Source, @this.Tokens[index]);
    }

    bool CheckText(string text) {
        return not @this.IsAtEnd() and @this.Text(@this.Current) == text;
    }

    bool MatchText(string text) {
        if (not @this.CheckText(text)) {
            return false;
        }
        @this.Current += 1;
        return true;
    }

    bool CheckKind(TokenModule.TokenKind kind) {
        return not @this.IsAtEnd() and @this.Tokens[@this.Current].Kind == kind;
    }

    bool MatchKind(TokenModule.TokenKind kind) {
        if (not @this.CheckKind(kind)) {
            return false;
        }
        @this.Current += 1;
        return true;
    }

    TokenModule.Token ExpectKind(TokenModule.TokenKind kind, string expectation) {
        if (@this.CheckKind(kind)) {
            TokenModule.Token token = @this.Tokens[@this.Current];
            @this.Current += 1;
            return token;
        }

        report(
            "expected " .: expectation .: " at " .: @this.Tokens[@this.Current].Line
                .: ":" .: @this.Tokens[@this.Current].Column
        );
        return @this.Tokens[@this.Current];
    }

    void ExpectText(string text, string expectation) {
        if (not @this.MatchText(text)) {
            report(
                "expected " .: expectation .: " at " .: @this.Tokens[@this.Current].Line
                    .: ":" .: @this.Tokens[@this.Current].Column
            );
        }
    }

    bool IsNamingTarget(string target) {
        return target == "functions"
            or target == "variables"
            or target == "parameters";
    }

    bool IsNamingConvention(string convention) {
        return convention == "UPPERCASE"
            or convention == "camelCase"
            or convention == "PascalCase"
            or convention == "UPPER_SNAKE_CASE";
    }

    bool HasNamingRule(array<Ast.NamingRule> rules, string target) {
        number index = 0;
        while (index < len(rules)) {
            if (rules[index].Target == target) {
                return true;
            }
            index += 1;
        }
        return false;
    }

    Ast.NamingRule ParseNamingRule() {
        TokenModule.Token targetToken = @this.ExpectKind(
            TokenModule.TokenKind::Identifier,
            "naming target"
        );
        string target = TokenModule.GetLexeme(@this.Source, targetToken);
        if (not @this.IsNamingTarget(target)) {
            report(
                "unknown naming target '" .: target
                    .: "'; expected functions, variables, or parameters"
            );
        }

        @this.ExpectKind(TokenModule.TokenKind::AssignmentOperator, "'=' after naming target");
        TokenModule.Token conventionToken = @this.ExpectKind(
            TokenModule.TokenKind::Identifier,
            "naming convention"
        );
        string convention = TokenModule.GetLexeme(@this.Source, conventionToken);
        if (not @this.IsNamingConvention(convention)) {
            report(
                "unknown naming convention '" .: convention
                    .: "'; expected UPPERCASE, camelCase, PascalCase, or UPPER_SNAKE_CASE"
            );
        }

        return Ast.NamingRule {
            Target = target,
            Convention = convention
        };
    }

    array<Ast.NamingRule> ParseStyleDirective(array<Ast.NamingRule> rules) {
        @this.ExpectText("!", "'!' before style directive");
        @this.ExpectText("style", "'style' after '!'");

        while (true) {
            Ast.NamingRule rule = @this.ParseNamingRule();
            if (@this.HasNamingRule(rules, rule.Target)) {
                report("duplicate naming target '" .: rule.Target .: "'");
            }
            push(rules, rule);
            if (not @this.MatchKind(TokenModule.TokenKind::Comma)) {
                break;
            }
        }

        @this.MatchKind(TokenModule.TokenKind::Semicolon);
        return rules;
    }

    Ast.Node NewNode(
        Ast.NodeKind kind,
        string name,
        string typeName,
        string value,
        number startToken,
        number endToken
    ) {
        return Ast.Node {
            Kind = kind,
            Name = name,
            TypeName = typeName,
            Value = value,
            IsExported = false,
            IsAuto = false,
            StartToken = startToken,
            EndToken = endToken,
            Children = array<Ast.Node>()
        };
    }

    Ast.Module ParseModule() {
        @this.ExpectText("!", "module directive '!as Name'");
        @this.ExpectText("as", "'as' after '!'");
        TokenModule.Token moduleName = @this.ExpectKind(
            TokenModule.TokenKind::Identifier,
            "module name"
        );
        Ast.Module module = Ast.Module {
            Name = TokenModule.GetLexeme(@this.Source, moduleName),
            NamingRules = array<Ast.NamingRule>(),
            Declarations = array<Ast.Node>()
        };

        @this.MatchKind(TokenModule.TokenKind::Semicolon);
        while (@this.CheckText("!") and @this.Current + 1 < len(@this.Tokens)
            and @this.Text(@this.Current + 1) == "style") {
            module.NamingRules = @this.ParseStyleDirective(module.NamingRules);
        }

        while (not @this.IsAtEnd()) {
            if (@this.CheckText("link")) {
                push(module.Declarations, @this.ParseLink());
            } else {
                push(module.Declarations, @this.ParseDeclarationOrStatement());
            }
        }

        return module;
    }

    Ast.Node ParseLink() {
        number startToken = @this.Current;
        @this.ExpectText("link", "'link'");
        TokenModule.Token path = @this.ExpectKind(
            TokenModule.TokenKind::StringLiteral,
            "quoted link path"
        );
        string alias = "";
        if (@this.MatchText("as")) {
            TokenModule.Token aliasToken = @this.ExpectKind(
                TokenModule.TokenKind::Identifier,
                "link alias"
            );
            alias = TokenModule.GetLexeme(@this.Source, aliasToken);
        }
        @this.ExpectKind(TokenModule.TokenKind::Semicolon, "';' after link declaration");

        return @this.NewNode(
            Ast.NodeKind::LinkDeclaration,
            alias,
            "",
            TokenModule.GetLexeme(@this.Source, path),
            startToken,
            @this.Current
        );
    }

    bool IsTypeWord(number index) {
        return @this.Tokens[index].Kind == TokenModule.TokenKind::Identifier
            or @this.Tokens[index].Kind == TokenModule.TokenKind::Keyword;
    }

    bool LooksLikeDeclaration(number index) {
        if (index >= len(@this.Tokens) or not @this.IsTypeWord(index)) {
            return false;
        }

        number cursor = index + 1;
        if (cursor < len(@this.Tokens) and @this.Text(cursor) == "<") {
            number genericDepth = 0;
            while (cursor < len(@this.Tokens)) {
                string text = @this.Text(cursor);
                if (text == "<") {
                    genericDepth += 1;
                } else if (text == ">") {
                    genericDepth -= 1;
                    if (genericDepth == 0) {
                        cursor += 1;
                        break;
                    }
                }
                cursor += 1;
            }
            if (genericDepth != 0) {
                return false;
            }
        }

        return cursor < len(@this.Tokens)
            and @this.Tokens[cursor].Kind == TokenModule.TokenKind::Identifier;
    }

    string ParseTypeName() {
        string typeName = @this.Text(@this.Current);
        if (not @this.IsTypeWord(@this.Current)) {
            report(
                "expected type name at " .: @this.Tokens[@this.Current].Line
                    .: ":" .: @this.Tokens[@this.Current].Column
            );
            return "";
        }
        @this.Current += 1;

        if (@this.MatchText("<")) {
            typeName = typeName .: "<" .: @this.ParseTypeName();
            while (@this.MatchKind(TokenModule.TokenKind::Comma)) {
                typeName = typeName .: "," .: @this.ParseTypeName();
            }
            @this.ExpectText(">", "'>' after generic type arguments");
            typeName = typeName .: ">";
        }

        return typeName;
    }

    bool IsArrayTypeName(string typeName) {
        return typeName == "array"
            or (
                len(typeName) >= len("array<")
                and typeName[0 .. len("array<")] == "array<"
            );
    }

    bool IsDictionaryTypeName(string typeName) {
        return typeName == "dictionary"
            or (
                len(typeName) >= len("dictionary<")
                and typeName[0 .. len("dictionary<")] == "dictionary<"
            );
    }

    bool IsCollectionTypeName(string typeName) {
        return @this.IsArrayTypeName(typeName)
            or @this.IsDictionaryTypeName(typeName);
    }

    Ast.Node CreateEmptyCollection(string typeName, number tokenIndex) {
        Ast.NodeKind kind = Ast.NodeKind::ArrayLiteral;
        if (@this.IsDictionaryTypeName(typeName)) {
            kind = Ast.NodeKind::DictionaryLiteral;
        }

        return @this.NewNode(kind, "", "", "", tokenIndex, tokenIndex);
    }

    Ast.Node ApplyCollectionType(Ast.Node initializer, string typeName) {
        if (
            initializer.Kind == Ast.NodeKind::ArrayLiteral
            and len(initializer.Children) == 0
            and @this.IsDictionaryTypeName(typeName)
        ) {
            initializer.Kind = Ast.NodeKind::DictionaryLiteral;
        }
        return initializer;
    }

    Ast.Node ParseDeclarationOrStatement() {
        bool isExported = @this.MatchText("export");
        number startToken = @this.Current;

        if (@this.MatchText("auto")) {
            TokenModule.Token nameToken = @this.ExpectKind(
                TokenModule.TokenKind::Identifier,
                "name after auto"
            );
            Ast.Node declaration = @this.NewNode(
                Ast.NodeKind::VariableDeclaration,
                TokenModule.GetLexeme(@this.Source, nameToken),
                "auto",
                "",
                startToken,
                @this.Current
            );
            declaration.IsAuto = true;
            declaration.IsExported = isExported;
            if (@this.MatchKind(TokenModule.TokenKind::AssignmentOperator)) {
                declaration.Children = array<Ast.Node>(@this.ParseExpression(0));
            }
            @this.ExpectKind(TokenModule.TokenKind::Semicolon, "';' after variable declaration");
            declaration.EndToken = @this.Current;
            return declaration;
        }

        if (isExported or @this.LooksLikeDeclaration(@this.Current)) {
            string typeName = @this.ParseTypeName();
            TokenModule.Token nameToken = @this.ExpectKind(
                TokenModule.TokenKind::Identifier,
                "declaration name"
            );
            string name = TokenModule.GetLexeme(@this.Source, nameToken);

            if (@this.MatchKind(TokenModule.TokenKind::LeftParen)) {
                Ast.Node function = @this.NewNode(
                    Ast.NodeKind::FunctionDeclaration,
                    name,
                    typeName,
                    "",
                    startToken,
                    @this.Current
                );
                function.IsExported = isExported;
                if (not @this.CheckKind(TokenModule.TokenKind::RightParen)) {
                    while (true) {
                        number parameterStart = @this.Current;
                        string parameterType = @this.ParseTypeName();
                        TokenModule.Token parameterName = @this.ExpectKind(
                            TokenModule.TokenKind::Identifier,
                            "parameter name"
                        );
                        push(function.Children, @this.NewNode(
                            Ast.NodeKind::Parameter,
                            TokenModule.GetLexeme(@this.Source, parameterName),
                            parameterType,
                            "",
                            parameterStart,
                            @this.Current
                        ));
                        if (not @this.MatchKind(TokenModule.TokenKind::Comma)) {
                            break;
                        }
                    }
                }
                @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after parameters");
                push(function.Children, @this.ParseBlock());
                function.EndToken = @this.Current;
                return function;
            }

            Ast.Node declaration = @this.NewNode(
                Ast.NodeKind::VariableDeclaration,
                name,
                typeName,
                "",
                startToken,
                @this.Current
            );
            declaration.IsExported = isExported;
            if (@this.MatchKind(TokenModule.TokenKind::AssignmentOperator)) {
                Ast.Node initializer = @this.ParseExpression(0);
                declaration.Children = array<Ast.Node>(
                    @this.ApplyCollectionType(initializer, typeName)
                );
            } else if (@this.IsCollectionTypeName(typeName)) {
                push(declaration.Children, @this.CreateEmptyCollection(typeName, @this.Current));
            }
            @this.ExpectKind(TokenModule.TokenKind::Semicolon, "';' after variable declaration");
            declaration.EndToken = @this.Current;
            return declaration;
        }

        if (isExported) {
            report("expected exported function or typed declaration");
        }
        return @this.ParseStatement();
    }

    Ast.Node ParseBlock() {
        number startToken = @this.Current;
        @this.ExpectKind(TokenModule.TokenKind::LeftBrace, "'{' to start block");
        Ast.Node block = @this.NewNode(
            Ast.NodeKind::Block,
            "",
            "",
            "",
            startToken,
            @this.Current
        );
        while (
            not @this.CheckKind(TokenModule.TokenKind::RightBrace)
            and not @this.IsAtEnd()
        ) {
            push(block.Children, @this.ParseStatement());
        }
        @this.ExpectKind(TokenModule.TokenKind::RightBrace, "'}' to close block");
        block.EndToken = @this.Current;
        return block;
    }

    Ast.Node ParseStatement() {
        if (@this.CheckKind(TokenModule.TokenKind::LeftBrace)) {
            return @this.ParseBlock();
        }

        number startToken = @this.Current;
        if (@this.MatchText("return")) {
            Ast.Node statement = @this.NewNode(
                Ast.NodeKind::ReturnStatement,
                "",
                "",
                "",
                startToken,
                @this.Current
            );
            if (not @this.CheckKind(TokenModule.TokenKind::Semicolon)) {
                statement.Children = array<Ast.Node>(@this.ParseExpression(0));
            }
            @this.ExpectKind(TokenModule.TokenKind::Semicolon, "';' after return");
            statement.EndToken = @this.Current;
            return statement;
        }

        if (
            @this.CheckText("auto")
            or @this.LooksLikeDeclaration(@this.Current)
        ) {
            return @this.ParseDeclarationOrStatement();
        }

        Ast.Node expression = @this.ParseExpression(0);
        @this.ExpectKind(TokenModule.TokenKind::Semicolon, "';' after expression");
        Ast.Node statement = @this.NewNode(
            Ast.NodeKind::ExpressionStatement,
            "",
            "",
            "",
            startToken,
            @this.Current
        );
        statement.Children = array<Ast.Node>(expression);
        return statement;
    }

    bool LooksLikeTypedCollection() {
        if (
            not @this.IsTypeWord(@this.Current)
            or @this.Current + 1 >= len(@this.Tokens)
            or @this.Text(@this.Current + 1) != "<"
        ) {
            return false;
        }

        number cursor = @this.Current + 1;
        number genericDepth = 0;
        while (cursor < len(@this.Tokens)) {
            string text = @this.Text(cursor);
            if (text == "<") {
                genericDepth += 1;
            } else if (text == ">") {
                genericDepth -= 1;
                if (genericDepth == 0) {
                    cursor += 1;
                    return cursor < len(@this.Tokens)
                        and @this.Tokens[cursor].Kind == TokenModule.TokenKind::LeftParen;
                }
            }
            cursor += 1;
        }
        return false;
    }

    number Precedence(string operator) {
        if (operator == "=") return 1;
        if (operator == "or") return 2;
        if (operator == "and") return 3;
        if (operator == "==" or operator == "!=" or operator == "<" or operator == "<=" or operator == ">" or operator == ">=") return 4;
        if (operator == "..") return 5;
        if (operator == "shl" or operator == "shr" or operator == "&" or operator == "|" or operator == "^") return 6;
        if (operator == "+" or operator == "-" or operator == ".:") return 7;
        if (operator == "*" or operator == "/" or operator == "%") return 8;
        return 0;
    }

    Ast.Node ParseExpression(number minimumPrecedence) {
        Ast.Node left = @this.ParsePrefix();

        while (not @this.IsAtEnd()) {
            if (@this.CheckKind(TokenModule.TokenKind::LeftParen)) {
                number startToken = left.StartToken;
                @this.Current += 1;
                Ast.Node call = @this.NewNode(
                    Ast.NodeKind::CallExpression,
                    "",
                    "",
                    "",
                    startToken,
                    @this.Current
                );
                push(call.Children, left);
                if (not @this.CheckKind(TokenModule.TokenKind::RightParen)) {
                    while (true) {
                        push(call.Children, @this.ParseExpression(0));
                        if (not @this.MatchKind(TokenModule.TokenKind::Comma)) {
                            break;
                        }
                    }
                }
                @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after arguments");
                call.EndToken = @this.Current;
                left = call;
                continue;
            }

            if (@this.MatchKind(TokenModule.TokenKind::Dot)) {
                number startToken = left.StartToken;
                TokenModule.Token member = @this.ExpectKind(
                    TokenModule.TokenKind::Identifier,
                    "member name after '.'"
                );
                Ast.Node access = @this.NewNode(
                    Ast.NodeKind::MemberExpression,
                    TokenModule.GetLexeme(@this.Source, member),
                    "",
                    "",
                    startToken,
                    @this.Current
                );
                push(access.Children, left);
                left = access;
                continue;
            }

            if (@this.CheckKind(TokenModule.TokenKind::LeftBracket)) {
                number startToken = left.StartToken;
                @this.Current += 1;
                Ast.Node indexExpression = @this.ParseExpression(0);
                @this.ExpectKind(TokenModule.TokenKind::RightBracket, "']' after index");
                Ast.Node access = @this.NewNode(
                    Ast.NodeKind::MemberExpression,
                    "[]",
                    "",
                    "",
                    startToken,
                    @this.Current
                );
                push(access.Children, left);
                push(access.Children, indexExpression);
                left = access;
                continue;
            }

            if (
                @this.CheckKind(TokenModule.TokenKind::IncrementOperator)
                and @this.Precedence("postfix") >= minimumPrecedence
            ) {
                string operator = @this.Text(@this.Current);
                number startToken = left.StartToken;
                @this.Current += 1;
                Ast.Node unary = @this.NewNode(
                    Ast.NodeKind::UnaryExpression,
                    operator,
                    "postfix",
                    "",
                    startToken,
                    @this.Current
                );
                push(unary.Children, left);
                left = unary;
                continue;
            }

            string operator = @this.Text(@this.Current);
            number precedence = @this.Precedence(operator);
            if (precedence == 0 or precedence < minimumPrecedence) {
                break;
            }

            number startToken = left.StartToken;
            @this.Current += 1;
            number nextMinimum = precedence + 1;
            if (operator == "=") {
                nextMinimum = precedence;
            }
            Ast.Node right = @this.ParseExpression(nextMinimum);
            Ast.NodeKind kind = Ast.NodeKind::BinaryExpression;
            if (operator == "=") {
                kind = Ast.NodeKind::Assignment;
            }
            Ast.Node binary = @this.NewNode(
                kind,
                operator,
                "",
                "",
                startToken,
                @this.Current
            );
            push(binary.Children, left);
            push(binary.Children, right);
            left = binary;
        }

        return left;
    }

    Ast.Node ParsePrefix() {
        number startToken = @this.Current;
        string text = @this.Text(@this.Current);

        if (text == "!" or text == "not" or text == "-" or text == "+" or text == "~") {
            @this.Current += 1;
            Ast.Node operand = @this.ParseExpression(9);
            Ast.Node unary = @this.NewNode(
                Ast.NodeKind::UnaryExpression,
                text,
                "prefix",
                "",
                startToken,
                @this.Current
            );
            push(unary.Children, operand);
            return unary;
        }

        if (@this.MatchKind(TokenModule.TokenKind::LeftParen)) {
            Ast.Node expression = @this.ParseExpression(0);
            @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after expression");
            return expression;
        }

        if (@this.LooksLikeTypedCollection()) {
            string typeName = @this.ParseTypeName();
            @this.ExpectKind(TokenModule.TokenKind::LeftParen, "'(' after collection type");
            Ast.Node initializer = @this.ParseExpression(0);
            @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after collection initializer");
            initializer = @this.ApplyCollectionType(initializer, typeName);
            Ast.Node typed = @this.NewNode(
                Ast.NodeKind::TypedCollectionExpression,
                "",
                typeName,
                "",
                startToken,
                @this.Current
            );
            push(typed.Children, initializer);
            return typed;
        }

        if (@this.CheckKind(TokenModule.TokenKind::LeftBrace)) {
            return @this.ParseCollectionLiteral();
        }

        if (@this.CheckKind(TokenModule.TokenKind::TemplateStart)) {
            number templateDepth = 0;
            while (not @this.IsAtEnd()) {
                if (@this.MatchKind(TokenModule.TokenKind::TemplateStart)) {
                    templateDepth += 1;
                } else if (@this.MatchKind(TokenModule.TokenKind::TemplateEnd)) {
                    templateDepth -= 1;
                    if (templateDepth == 0) {
                        break;
                    }
                } else {
                    @this.Current += 1;
                }
            }
            Ast.Node template = @this.NewNode(
                Ast.NodeKind::LiteralExpression,
                "template",
                "string",
                "",
                startToken,
                @this.Current
            );
            return template;
        }

        TokenModule.Token token = @this.Tokens[@this.Current];
        if (
            token.Kind == TokenModule.TokenKind::DecimalIntegerLiteral
            or token.Kind == TokenModule.TokenKind::BinaryIntegerLiteral
            or token.Kind == TokenModule.TokenKind::HexIntegerLiteral
            or token.Kind == TokenModule.TokenKind::FloatLiteral
            or token.Kind == TokenModule.TokenKind::StringLiteral
            or token.Kind == TokenModule.TokenKind::BooleanLiteral
            or token.Kind == TokenModule.TokenKind::NullLiteral
        ) {
            @this.Current += 1;
            return @this.NewNode(
                Ast.NodeKind::LiteralExpression,
                "",
                "",
                TokenModule.GetLexeme(@this.Source, token),
                startToken,
                @this.Current
            );
        }

        if (token.Kind == TokenModule.TokenKind::Identifier or token.Kind == TokenModule.TokenKind::Keyword) {
            @this.Current += 1;
            return @this.NewNode(
                Ast.NodeKind::IdentifierExpression,
                TokenModule.GetLexeme(@this.Source, token),
                "",
                "",
                startToken,
                @this.Current
            );
        }

        report(
            "expected expression at " .: token.Line .: ":" .: token.Column
        );
        return @this.NewNode(
            Ast.NodeKind::LiteralExpression,
            "",
            "",
            "",
            startToken,
            startToken
        );
    }

    Ast.Node ParseCollectionLiteral() {
        number startToken = @this.Current;
        @this.ExpectKind(TokenModule.TokenKind::LeftBrace, "'{' to start collection literal");
        bool isDictionary = false;
        if (
            @this.CheckKind(TokenModule.TokenKind::LeftBracket)
            or @this.Tokens[@this.Current].Kind == TokenModule.TokenKind::Identifier
            or @this.Tokens[@this.Current].Kind == TokenModule.TokenKind::StringLiteral
        ) {
            number lookahead = @this.Current;
            if (@this.Tokens[lookahead].Kind == TokenModule.TokenKind::LeftBracket) {
                while (lookahead < len(@this.Tokens) and @this.Text(lookahead) != "]") {
                    lookahead += 1;
                }
                isDictionary = lookahead + 1 < len(@this.Tokens) and @this.Text(lookahead + 1) == "=";
            } else {
                isDictionary = lookahead + 1 < len(@this.Tokens)
                    and @this.Text(lookahead + 1) == "=";
            }
        }

        Ast.Node literal = @this.NewNode(
            isDictionary ? Ast.NodeKind::DictionaryLiteral : Ast.NodeKind::ArrayLiteral,
            "",
            "",
            "",
            startToken,
            @this.Current
        );

        if (not @this.CheckKind(TokenModule.TokenKind::RightBrace)) {
            while (true) {
                if (isDictionary) {
                    Ast.Node key;
                    if (@this.MatchKind(TokenModule.TokenKind::LeftBracket)) {
                        key = @this.ParseExpression(0);
                        @this.ExpectKind(TokenModule.TokenKind::RightBracket, "']' after dictionary key");
                    } else {
                        key = @this.ParseExpression(2);
                    }
                    @this.ExpectKind(TokenModule.TokenKind::AssignmentOperator, "'=' after dictionary key");
                    Ast.Node value = @this.ParseExpression(0);
                    Ast.Node entry = @this.NewNode(
                        Ast.NodeKind::ExpressionStatement,
                        "entry",
                        "",
                        "",
                        key.StartToken,
                        value.EndToken
                    );
                    push(entry.Children, key);
                    push(entry.Children, value);
                    push(literal.Children, entry);
                } else {
                    push(literal.Children, @this.ParseExpression(0));
                }

                if (not @this.MatchKind(TokenModule.TokenKind::Comma)) {
                    break;
                }
                if (@this.CheckKind(TokenModule.TokenKind::RightBrace)) {
                    break;
                }
            }
        }

        @this.ExpectKind(TokenModule.TokenKind::RightBrace, "'}' after collection literal");
        literal.EndToken = @this.Current;
        return literal;
    }
}

export Ast.Module Parse(string source, array<TokenModule.Token> tokens) {
    Parser parser = Parser {
        Source = source,
        Tokens = tokens,
        Current = 0
    };
    return parser.ParseModule();
}
