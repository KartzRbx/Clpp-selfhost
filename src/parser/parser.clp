link "../ast/ast.clp" as Ast;
link @clpp.text as Text
link "../lexer/token.clp" as TokenModule;

const array<string> NUMERIC_TYPES = array<string>(
    "number", "float", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f16", "f32", "f64"
)

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

    void ExpectStatementTerminator(string statement) {
        if (@this.MatchKind(TokenModule.TokenKind::Semicolon)) {
            return;
        }

        if (
            @this.IsAtEnd()
            or @this.CheckKind(TokenModule.TokenKind::RightBrace)
            or @this.Tokens[@this.Current].Line > @this.Tokens[@this.Current - 1].Line
        ) {
            return;
        }

        report(
            "expected ';' or line break after " .: statement .: " at "
                .: @this.Tokens[@this.Current].Line .: ":" .: @this.Tokens[@this.Current].Column
        );
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
            IsConst = false,
            StartToken = startToken,
            EndToken = endToken,
            Children = array<Ast.Node>(),
            Docs = array<Ast.DocTag>()
        };
    }

    bool IsDocTag(string name) {
        return name == "name"
            or name == "desc"
            or name == "params"
            or name == "return"
            or name == "example"
            or name == "deprecated"
            or name == "see";
    }

    << <<{
    <<     !name = "Distance"
    <<     !desc = "Euclidean distance between two points"
    << }>>
    array<Ast.DocTag> ParseDocComment() {
        @ExpectKind(TokenModule.TokenKind::DocCommentStart, "'<<{' to start doc comment")
        array<Ast.DocTag> tags = array<Ast.DocTag>()
        while (
            not @CheckKind(TokenModule.TokenKind::DocCommentEnd)
            and not @IsAtEnd()
        ) {
            @ExpectText("!", "'!' before doc tag")
            TokenModule.Token tagToken = @Tokens[@Current]
            if (not @IsTypeWord(@Current)) {
                report("expected doc tag name at " .: tagToken.Line .: ":" .: tagToken.Column)
            }
            @Current += 1
            string tagName = TokenModule.GetLexeme(@Source, tagToken)
            if (not @IsDocTag(tagName)) {
                report(
                    "unknown doc tag '!" .: tagName .: "' at " .: tagToken.Line .: ":" .: tagToken.Column
                        .: "; expected name, desc, params, return, example, deprecated or see"
                )
            }
            for (Ast.DocTag existing in tags) {
                if (existing.Name == tagName) {
                    report("duplicate doc tag '!" .: tagName .: "' at " .: tagToken.Line .: ":" .: tagToken.Column)
                }
            }
            @ExpectText("=", "'=' after doc tag name")
            TokenModule.Token valueToken = @ExpectKind(
                TokenModule.TokenKind::StringLiteral,
                "quoted doc tag value"
            )
            string quoted = TokenModule.GetLexeme(@Source, valueToken)
            push(tags, Ast.DocTag {
                Name = tagName,
                Value = quoted[1 .. len(quoted) - 1]
            })
            @MatchKind(TokenModule.TokenKind::Semicolon)
            @MatchKind(TokenModule.TokenKind::Comma)
        }
        @ExpectKind(TokenModule.TokenKind::DocCommentEnd, "'}>>' to close doc comment")
        return tags
    }

    Ast.Node ParseDocumentedDeclaration() {
        TokenModule.Token docToken = @Tokens[@Current]
        array<Ast.DocTag> tags = @ParseDocComment()
        if (@CheckKind(TokenModule.TokenKind::DocCommentStart)) {
            report("two doc comments in a row at " .: docToken.Line .: ":" .: docToken.Column)
        }
        Ast.Node declaration = @ParseDeclarationOrStatement()
        if (
            declaration.Kind != Ast.NodeKind::FunctionDeclaration
            and declaration.Kind != Ast.NodeKind::VariableDeclaration
            and declaration.Kind != Ast.NodeKind::TypeDeclaration
            and declaration.Kind != Ast.NodeKind::DefDeclaration
            and declaration.Kind != Ast.NodeKind::EnumDeclaration
        ) {
            report(
                "doc comment at " .: docToken.Line .: ":" .: docToken.Column
                    .: " must be followed by a function, variable, def or enum declaration"
            )
        }
        for (Ast.DocTag tag in tags) {
            if (tag.Name == "name" and tag.Value != declaration.Name) {
                report(
                    "doc comment names '" .: tag.Value .: "' but documents '"
                        .: declaration.Name .: "' at " .: docToken.Line .: ":" .: docToken.Column
                )
            }
        }
        declaration.Docs = tags
        return declaration
    }

    Ast.Module ParseModule() {
        << `!as Name` is optional; without it the module has no declared name.
        string moduleName = ""
        if (
            @CheckText("!")
            and @Current + 1 < len(@Tokens)
            and @Text(@Current + 1) == "as"
        ) {
            @Current += 2
            moduleName = TokenModule.GetLexeme(@Source, @ExpectKind(
                TokenModule.TokenKind::Identifier,
                "module name after '!as'"
            ))
            @MatchKind(TokenModule.TokenKind::Semicolon)
        }
        Ast.Module module = Ast.Module {
            Name = moduleName,
            NamingRules = array<Ast.NamingRule>(),
            Declarations = array<Ast.Node>()
        };
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
        string target = ""
        if (@MatchKind(TokenModule.TokenKind::AtSign)) {
            << Standard module: link @clpp.fs as Fs
            target = "@" .: TokenModule.GetLexeme(@Source, @ExpectKind(
                TokenModule.TokenKind::Identifier,
                "standard module name after '@'"
            ))
            while (@MatchKind(TokenModule.TokenKind::MemberAccessOperator)) {
                target = target .: "." .: TokenModule.GetLexeme(@Source, @ExpectKind(
                    TokenModule.TokenKind::Identifier,
                    "module name after '.'"
                ))
            }
        } else {
            target = TokenModule.GetLexeme(@Source, @ExpectKind(
                TokenModule.TokenKind::StringLiteral,
                "quoted link path or @clpp module"
            ))
        }
        string alias = "";
        if (@this.MatchText("as")) {
            TokenModule.Token aliasToken = @this.ExpectKind(
                TokenModule.TokenKind::Identifier,
                "link alias"
            );
            alias = TokenModule.GetLexeme(@this.Source, aliasToken);
        }
        @this.ExpectStatementTerminator("link declaration");

        return @this.NewNode(
            Ast.NodeKind::LinkDeclaration,
            alias,
            "",
            target,
            startToken,
            @this.Current
        );
    }

    bool IsTypeWord(number index) {
        return @this.Tokens[index].Kind == TokenModule.TokenKind::Identifier
            or @this.Tokens[index].Kind == TokenModule.TokenKind::Keyword;
    }

    bool IsStatementKeyword(string text) {
        return text == "if"
            or text == "else"
            or text == "while"
            or text == "do"
            or text == "for"
            or text == "guard"
            or text == "try"
            or text == "match"
            or text == "catch"
            or text == "break"
            or text == "continue"
            or text == "return";
    }

    bool LooksLikeDeclaration(number index) {
        if (
            index >= len(@this.Tokens)
            or not @this.IsTypeWord(index)
            or @this.IsStatementKeyword(@this.Text(index))
        ) {
            return false;
        }

        number cursor = index + 1;
        while (
            cursor + 1 < len(@this.Tokens)
            and @this.Tokens[cursor].Kind == TokenModule.TokenKind::MemberAccessOperator
            and @this.IsTypeWord(cursor + 1)
        ) {
            cursor += 2;
        }
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

        << Without semicolons the type and the name must share a line:
        << `break` followed by `index += 1` on the next line is not a declaration.
        return cursor < len(@this.Tokens)
            and @this.Tokens[cursor].Kind == TokenModule.TokenKind::Identifier
            and @this.Tokens[cursor].Line == @this.Tokens[cursor - 1].Line;
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

        while (
            @this.CheckKind(TokenModule.TokenKind::MemberAccessOperator)
            and @this.IsTypeWord(@this.Current + 1)
        ) {
            typeName = typeName .: "." .: @this.Text(@this.Current + 1);
            @this.Current += 2;
        }

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

    Ast.Node ParseTypeExpression() {
        Ast.Node left = @this.ParseTypePrimary();
        while (@this.CheckText("&")) {
            @this.Current += 1;
            Ast.Node intersection = @this.NewNode(
                Ast.NodeKind::TypeIntersection,
                "&",
                "",
                "",
                left.StartToken,
                @this.Current
            );
            push(intersection.Children, left);
            push(intersection.Children, @this.ParseTypePrimary());
            intersection.EndToken = @this.Current;
            left = intersection;
        }
        return left;
    }

    Ast.Node ParseTypePrimary() {
        number startToken = @this.Current;

        if (@this.MatchText("typeof")) {
            @this.ExpectKind(TokenModule.TokenKind::LeftParen, "'(' after typeof");
            Ast.Node expression = @this.ParseExpression(0);
            @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after typeof expression");
            Ast.Node typeOf = @this.NewNode(
                Ast.NodeKind::TypeofType,
                "",
                "",
                "",
                startToken,
                @this.Current
            );
            push(typeOf.Children, expression);
            return typeOf;
        }

        if (@this.CheckKind(TokenModule.TokenKind::LeftBrace)) {
            @this.Current += 1;
            Ast.Node objectType = @this.NewNode(
                Ast.NodeKind::ObjectType,
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
                number fieldStart = @this.Current;
                TokenModule.Token fieldName = @this.ExpectKind(
                    TokenModule.TokenKind::Identifier,
                    "field name in object type"
                );
                @this.ExpectKind(TokenModule.TokenKind::Colon, "':' after object type field");
                Ast.Node fieldType = @this.ParseTypeExpression();
                Ast.Node field = @this.NewNode(
                    Ast.NodeKind::TypeField,
                    TokenModule.GetLexeme(@this.Source, fieldName),
                    "",
                    "",
                    fieldStart,
                    @this.Current
                );
                push(field.Children, fieldType);
                push(objectType.Children, field);

                if (
                    @this.MatchKind(TokenModule.TokenKind::Comma)
                    or @this.MatchKind(TokenModule.TokenKind::Semicolon)
                ) {
                    continue;
                }
                if (@this.CheckKind(TokenModule.TokenKind::RightBrace)) {
                    break;
                }
                if (
                    @this.IsAtEnd()
                    or @this.Tokens[@this.Current].Line
                        > @this.Tokens[field.EndToken - 1].Line
                ) {
                    continue;
                }
                report(
                    "expected ',' or line break after object type field at "
                        .: @this.Tokens[@this.Current].Line
                        .: ":" .: @this.Tokens[@this.Current].Column
                );
                break;
            }
            @this.ExpectKind(TokenModule.TokenKind::RightBrace, "'}' after object type");
            objectType.EndToken = @this.Current;
            return objectType;
        }

        TokenModule.Token typeToken = @this.Tokens[@this.Current];
        if (not @this.IsTypeWord(@this.Current)) {
            report(
                "expected type name at " .: typeToken.Line .: ":" .: typeToken.Column
            );
        }
        @this.Current += 1;
        string typeName = TokenModule.GetLexeme(@this.Source, typeToken);
        Ast.Node typeReference = @this.NewNode(
            Ast.NodeKind::TypeReference,
            "",
            typeName,
            "",
            startToken,
            @this.Current
        );
        if (@this.MatchText("<")) {
            while (true) {
                push(typeReference.Children, @this.ParseTypeExpression());
                if (not @this.MatchKind(TokenModule.TokenKind::Comma)) {
                    break;
                }
            }
            @this.ExpectText(">", "'>' after type arguments");
            typeReference.EndToken = @this.Current;
        }
        return typeReference;
    }

    << <T, U: Bound> or <typename T>; returns "T,U".
    string ParseTypeParameters() {
        @ExpectText("<", "'<' before type parameters")
        array<string> names = array<string>()
        while (true) {
            @MatchText("typename")
            push(names, TokenModule.GetLexeme(@Source, @ExpectKind(
                TokenModule.TokenKind::Identifier,
                "type parameter name"
            )))
            if (@MatchKind(TokenModule.TokenKind::Colon)) {
                @ParseTypeName()
            }
            if (not @MatchKind(TokenModule.TokenKind::Comma)) {
                break
            }
        }
        @ExpectText(">", "'>' after type parameters")
        string joined = ""
        for (string typeName in names) {
            if (joined != "") {
                joined = joined .: ","
            }
            joined = joined .: typeName
        }
        return joined
    }

    << Box<number> { ... }: a generic type followed by an object body.
    bool LooksLikeGenericConstruction() {
        if (
            not @IsTypeWord(@Current)
            or @Current + 1 >= len(@Tokens)
            or @Text(@Current + 1) != "<"
        ) {
            return false
        }
        number cursor = @Current + 1
        number depth = 0
        while (cursor < len(@Tokens)) {
            string text = @Text(cursor)
            if (text == "<") {
                depth += 1
            } else if (text == ">") {
                depth -= 1
                if (depth == 0) {
                    return cursor + 1 < len(@Tokens)
                        and @Tokens[cursor + 1].Kind == TokenModule.TokenKind::LeftBrace
                        and @Tokens[cursor + 1].Line == @Tokens[cursor].Line
                }
            } else if (not @IsTypeWord(cursor) and text != "." and text != ",") {
                return false
            }
            cursor += 1
        }
        return false
    }

    Ast.Node ParseTypeDeclaration(bool isExported, number startToken) {
        @this.ExpectText("def", "'def' before type name");
        TokenModule.Token name = @this.ExpectKind(
            TokenModule.TokenKind::Identifier,
            "type name after 'def'"
        );
        << def Box<T> { T Value }: the type parameters are kept in TypeName.
        string typeParameters = ""
        if (@CheckText("<")) {
            typeParameters = @ParseTypeParameters()
        }
        if (@CheckKind(TokenModule.TokenKind::LeftBrace)) {
            Ast.Node definition = @ParseDefBody(TokenModule.GetLexeme(@Source, name), isExported, startToken)
            definition.TypeName = typeParameters
            return definition
        }
        if (@CheckKind(TokenModule.TokenKind::Colon)) {
            report(
                "def inheritance is not supported yet at "
                    .: @Tokens[@Current].Line .: ":" .: @Tokens[@Current].Column
            )
        }
        @this.ExpectKind(TokenModule.TokenKind::AssignmentOperator, "'=' or '{' after type name");
        Ast.Node typeExpression = @this.ParseTypeExpression();
        Ast.Node declaration = @this.NewNode(
            Ast.NodeKind::TypeDeclaration,
            TokenModule.GetLexeme(@this.Source, name),
            "",
            "",
            startToken,
            @this.Current
        );
        declaration.IsExported = isExported;
        push(declaration.Children, typeExpression);
        @this.ExpectStatementTerminator("type declaration");
        declaration.EndToken = @this.Current;
        return declaration;
    }

    << def Name {
    <<     Type Field
    <<     Type Field = default
    <<     Type Method(Type parameter) { }
    << }
    << Children: FieldDeclaration and FunctionDeclaration nodes, in source order.
    Ast.Node ParseDefBody(string name, bool isExported, number startToken) {
        Ast.Node definition = @NewNode(Ast.NodeKind::DefDeclaration, name, "", "", startToken, @Current)
        definition.IsExported = isExported
        @ExpectKind(TokenModule.TokenKind::LeftBrace, "'{' to start def body")

        while (not @CheckKind(TokenModule.TokenKind::RightBrace) and not @IsAtEnd()) {
            if (@MatchKind(TokenModule.TokenKind::Semicolon)) {
                continue
            }
            array<Ast.DocTag> docs = array<Ast.DocTag>()
            if (@CheckKind(TokenModule.TokenKind::DocCommentStart)) {
                docs = @ParseDocComment()
            }
            number memberStart = @Current
            string visibility = "public"
            if (@MatchText("private")) {
                visibility = "private"
            } else {
                @MatchText("public")
            }

            string typeName = @ParseTypeName()
            TokenModule.Token nameToken = @ExpectKind(
                TokenModule.TokenKind::Identifier,
                "field or method name in def " .: name
            )
            string memberName = TokenModule.GetLexeme(@Source, nameToken)
            Ast.Node member = @NewNode(
                Ast.NodeKind::FieldDeclaration,
                memberName,
                typeName,
                visibility,
                memberStart,
                @Current
            )
            if (@MatchKind(TokenModule.TokenKind::LeftParen)) {
                member = @ParseFunctionRest(memberName, typeName, false, memberStart)
                member.Value = visibility
            } else {
                if (@MatchText("=")) {
                    push(member.Children, @ParseExpression(0))
                }
                @ExpectStatementTerminator("field declaration")
                member.EndToken = @Current
            }

            for (Ast.Node existing in definition.Children) {
                if (existing.Name == memberName) {
                    report(
                        "def " .: name .: " declares '" .: memberName .: "' more than once at "
                            .: nameToken.Line .: ":" .: nameToken.Column
                    )
                }
            }
            member.Docs = docs
            push(definition.Children, member)
        }

        @ExpectKind(TokenModule.TokenKind::RightBrace, "'}' to close def " .: name)
        @MatchKind(TokenModule.TokenKind::Semicolon)
        definition.EndToken = @Current
        return definition
    }

    << enum class Name { First, Second }
    Ast.Node ParseEnumDeclaration(bool isExported, number startToken) {
        @ExpectText("enum", "'enum'")
        if (not @MatchText("class")) {
            report(
                "expected 'class' after 'enum' at " .: @Tokens[@Current].Line .: ":" .: @Tokens[@Current].Column
                    .: "; CL++ only has scoped enums: enum class Name { ... }"
            )
        }
        TokenModule.Token nameToken = @ExpectKind(TokenModule.TokenKind::Identifier, "enum name")
        Ast.Node declaration = @NewNode(
            Ast.NodeKind::EnumDeclaration,
            TokenModule.GetLexeme(@Source, nameToken),
            "",
            "",
            startToken,
            @Current
        )
        declaration.IsExported = isExported
        @ExpectKind(TokenModule.TokenKind::LeftBrace, "'{' after enum name")

        while (not @CheckKind(TokenModule.TokenKind::RightBrace) and not @IsAtEnd()) {
            TokenModule.Token memberToken = @ExpectKind(TokenModule.TokenKind::Identifier, "enum member name")
            string memberName = TokenModule.GetLexeme(@Source, memberToken)
            for (Ast.Node existing in declaration.Children) {
                if (existing.Name == memberName) {
                    report(
                        "enum class " .: declaration.Name .: " declares '" .: memberName
                            .: "' more than once at " .: memberToken.Line .: ":" .: memberToken.Column
                    )
                }
            }
            push(declaration.Children, @NewNode(
                Ast.NodeKind::EnumMember,
                memberName,
                "",
                "",
                @Current - 1,
                @Current
            ))
            if (not @MatchKind(TokenModule.TokenKind::Comma)) {
                break
            }
        }

        @ExpectKind(TokenModule.TokenKind::RightBrace, "'}' to close enum " .: declaration.Name)
        @MatchKind(TokenModule.TokenKind::Semicolon)
        declaration.EndToken = @Current
        return declaration
    }

    << Parses `(parameters) { body }` after a function name.
    Ast.Node ParseFunctionRest(string name, string typeName, bool isExported, number startToken) {
        Ast.Node function = @NewNode(
            Ast.NodeKind::FunctionDeclaration,
            name,
            typeName,
            "",
            startToken,
            @Current
        )
        function.IsExported = isExported
        if (not @CheckKind(TokenModule.TokenKind::RightParen)) {
            while (true) {
                number parameterStart = @Current
                string parameterType = @ParseTypeName()
                TokenModule.Token parameterName = @ExpectKind(
                    TokenModule.TokenKind::Identifier,
                    "parameter name"
                )
                push(function.Children, @NewNode(
                    Ast.NodeKind::Parameter,
                    TokenModule.GetLexeme(@Source, parameterName),
                    parameterType,
                    "",
                    parameterStart,
                    @Current
                ))
                if (not @MatchKind(TokenModule.TokenKind::Comma)) {
                    break
                }
            }
        }
        @ExpectKind(TokenModule.TokenKind::RightParen, "')' after parameters")
        push(function.Children, @ParseBlock())
        function.EndToken = @Current
        return function
    }

    Ast.Node ParseDeclarationOrStatement() {
        if (@CheckKind(TokenModule.TokenKind::DocCommentStart)) {
            return @ParseDocumentedDeclaration()
        }
        bool isExported = @this.MatchText("export");
        << template<typename T> T First(array<T> values): parameters matter to the checker only.
        if (@MatchText("template")) {
            @ParseTypeParameters()
        }
        number startToken = @this.Current;

        if (@this.CheckText("def")) {
            return @this.ParseTypeDeclaration(isExported, startToken);
        }
        if (@CheckText("enum")) {
            return @ParseEnumDeclaration(isExported, startToken)
        }

        bool isConst = @this.MatchText("const");

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
            declaration.IsConst = isConst;
            declaration.IsExported = isExported;
            if (@this.MatchText("=")) {
                declaration.Children = array<Ast.Node>(@this.ParseExpression(0));
            } else if (isConst) {
                @this.ReportConstWithoutValue(declaration.Name);
            }
            @this.ExpectStatementTerminator("variable declaration");
            declaration.EndToken = @this.Current;
            return declaration;
        }

        if (isExported or isConst or @this.LooksLikeDeclaration(@this.Current)) {
            string typeName = @this.ParseTypeName();
            TokenModule.Token nameToken = @this.ExpectKind(
                TokenModule.TokenKind::Identifier,
                "declaration name"
            );
            string name = TokenModule.GetLexeme(@this.Source, nameToken);

            if (@this.MatchKind(TokenModule.TokenKind::LeftParen)) {
                if (isConst) {
                    report("function '" .: name .: "' cannot be declared const");
                }
                return @ParseFunctionRest(name, typeName, isExported, startToken)
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
            declaration.IsConst = isConst;
            if (@this.MatchText("=")) {
                Ast.Node initializer = @this.ParseExpression(0);
                declaration.Children = array<Ast.Node>(
                    @this.ApplyCollectionType(initializer, typeName)
                );
            } else if (isConst) {
                @this.ReportConstWithoutValue(name);
            } else if (@this.IsCollectionTypeName(typeName)) {
                push(declaration.Children, @this.CreateEmptyCollection(typeName, @this.Current));
            }
            @this.ExpectStatementTerminator("variable declaration");
            declaration.EndToken = @this.Current;
            return declaration;
        }

        if (isExported) {
            report("expected exported function or typed declaration");
        }
        return @this.ParseStatement();
    }

    void ReportConstWithoutValue(string name) {
        report(
            "const declaration '" .: name .: "' requires an initializer at "
                .: @this.Tokens[@this.Current].Line .: ":" .: @this.Tokens[@this.Current].Column
        );
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
        if (@CheckKind(TokenModule.TokenKind::DocCommentStart)) {
            return @ParseDocumentedDeclaration()
        }
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
            if (
                not @this.CheckKind(TokenModule.TokenKind::Semicolon)
                and not @this.IsAtEnd()
                and not @this.CheckKind(TokenModule.TokenKind::RightBrace)
                and @this.Tokens[@this.Current].Line == @this.Tokens[@this.Current - 1].Line
            ) {
                statement.Children = array<Ast.Node>(@this.ParseExpression(0));
            }
            @this.ExpectStatementTerminator("return");
            statement.EndToken = @this.Current;
            return statement;
        }

        if (@this.CheckText("if")) {
            return @this.ParseIf();
        }
        if (@this.CheckText("while")) {
            return @this.ParseWhile();
        }
        if (@this.CheckText("do")) {
            return @this.ParseDoWhile();
        }
        if (@this.CheckText("for")) {
            return @this.ParseFor();
        }
        if (@this.CheckText("guard")) {
            return @this.ParseGuard();
        }
        if (@this.CheckText("try")) {
            return @this.ParseTry();
        }
        if (@CheckText("match")) {
            return @ParseMatch()
        }
        if (@this.CheckText("break") or @this.CheckText("continue")) {
            string keyword = @this.Text(@this.Current);
            @this.Current += 1;
            Ast.NodeKind jumpKind = Ast.NodeKind::BreakStatement;
            if (keyword == "continue") {
                jumpKind = Ast.NodeKind::ContinueStatement;
            }
            @this.ExpectStatementTerminator(keyword);
            return @this.NewNode(jumpKind, keyword, "", "", startToken, @this.Current);
        }
        if (@this.CheckText("else") or @this.CheckText("catch")) {
            report(
                "'" .: @this.Text(@this.Current) .: "' without a matching statement at "
                    .: @this.Tokens[@this.Current].Line .: ":" .: @this.Tokens[@this.Current].Column
            );
            @this.Current += 1;
        }

        if (
            @this.CheckText("auto")
            or @this.CheckText("const")
            or @this.LooksLikeDeclaration(@this.Current)
        ) {
            return @this.ParseDeclarationOrStatement();
        }

        Ast.Node expression = @this.ParseExpression(0);
        @this.ExpectStatementTerminator("expression");
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

    Ast.Node ParseCondition(string statement) {
        @this.ExpectKind(TokenModule.TokenKind::LeftParen, "'(' after " .: statement);
        Ast.Node condition = @this.ParseExpression(0);
        @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after " .: statement .: " condition");
        return condition;
    }

    << if (cond) { } [else if (cond) { }] [else { }]
    << Children: condition, then-block, optional else (Block or IfStatement).
    Ast.Node ParseIf() {
        number startToken = @this.Current;
        @this.ExpectText("if", "'if'");
        Ast.Node statement = @this.NewNode(Ast.NodeKind::IfStatement, "", "", "", startToken, @this.Current);
        push(statement.Children, @this.ParseCondition("if"));
        push(statement.Children, @this.ParseBlock());
        if (@this.MatchText("else")) {
            if (@this.CheckText("if")) {
                push(statement.Children, @this.ParseIf());
            } else {
                push(statement.Children, @this.ParseBlock());
            }
        }
        statement.EndToken = @this.Current;
        return statement;
    }

    << while (cond) { }  -> Children: condition, body.
    Ast.Node ParseWhile() {
        number startToken = @this.Current;
        @this.ExpectText("while", "'while'");
        Ast.Node statement = @this.NewNode(Ast.NodeKind::WhileStatement, "", "", "", startToken, @this.Current);
        push(statement.Children, @this.ParseCondition("while"));
        push(statement.Children, @this.ParseBlock());
        statement.EndToken = @this.Current;
        return statement;
    }

    << do { } while (cond)  -> Children: body, condition.
    Ast.Node ParseDoWhile() {
        number startToken = @this.Current;
        @this.ExpectText("do", "'do'");
        Ast.Node statement = @this.NewNode(Ast.NodeKind::DoWhileStatement, "", "", "", startToken, @this.Current);
        push(statement.Children, @this.ParseBlock());
        @this.ExpectText("while", "'while' after do block");
        push(statement.Children, @this.ParseCondition("do-while"));
        @this.ExpectStatementTerminator("do-while");
        statement.EndToken = @this.Current;
        return statement;
    }

    << for (Type item in values) { }
    << Children: loop variable (VariableDeclaration), iterable, body.
    Ast.Node ParseFor() {
        number startToken = @this.Current;
        @this.ExpectText("for", "'for'");
        @this.ExpectKind(TokenModule.TokenKind::LeftParen, "'(' after for");

        number variableStart = @this.Current;
        bool isAuto = @this.MatchText("auto");
        string typeName = "auto";
        if (not isAuto) {
            typeName = @this.ParseTypeName();
        }
        TokenModule.Token nameToken = @this.ExpectKind(
            TokenModule.TokenKind::Identifier,
            "loop variable name"
        );
        Ast.Node variable = @this.NewNode(
            Ast.NodeKind::VariableDeclaration,
            TokenModule.GetLexeme(@this.Source, nameToken),
            typeName,
            "",
            variableStart,
            @this.Current
        );
        variable.IsAuto = isAuto;

        if (not @this.MatchText("in")) {
            report(
                "expected 'in' after loop variable at " .: @this.Tokens[@this.Current].Line
                    .: ":" .: @this.Tokens[@this.Current].Column
                    .: "; CL++ loops use 'for (Type item in values)', use while for counters"
            );
        }
        Ast.Node iterable = @this.ParseExpression(0);
        @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after for iterable");

        Ast.Node statement = @this.NewNode(Ast.NodeKind::ForInStatement, "", "", "", startToken, @this.Current);
        push(statement.Children, variable);
        push(statement.Children, iterable);
        push(statement.Children, @this.ParseBlock());
        statement.EndToken = @this.Current;
        return statement;
    }

    << guard (cond) else { }  -> Children: condition, else-block.
    Ast.Node ParseGuard() {
        number startToken = @this.Current;
        @this.ExpectText("guard", "'guard'");
        Ast.Node statement = @this.NewNode(Ast.NodeKind::GuardStatement, "", "", "", startToken, @this.Current);
        push(statement.Children, @this.ParseCondition("guard"));
        @this.ExpectText("else", "'else' after guard condition");
        push(statement.Children, @this.ParseBlock());
        statement.EndToken = @this.Current;
        return statement;
    }

    << `a ${b} c` becomes "a " .: b .: " c" (a chain of BinaryExpression ".:").
    Ast.Node ParseTemplate() {
        number startToken = @Current
        @ExpectKind(TokenModule.TokenKind::TemplateStart, "template string")
        Ast.Node result = @NewNode(Ast.NodeKind::LiteralExpression, "", "", "\"\"", startToken, @Current)
        while (not @CheckKind(TokenModule.TokenKind::TemplateEnd) and not @IsAtEnd()) {
            Ast.Node part = result
            if (@CheckKind(TokenModule.TokenKind::TemplateText)) {
                part = @NewNode(
                    Ast.NodeKind::LiteralExpression,
                    "",
                    "",
                    @TemplateTextLiteral(@Text(@Current)),
                    @Current,
                    @Current + 1
                )
                @Current += 1
            } else {
                @ExpectKind(TokenModule.TokenKind::TemplateInterpolationStart, "'${' or text in template string")
                part = @ParseExpression(0)
                @ExpectKind(TokenModule.TokenKind::TemplateInterpolationEnd, "'}' to close ${ in template string")
            }
            Ast.Node joined = @NewNode(Ast.NodeKind::BinaryExpression, ".:", "", "", startToken, @Current)
            push(joined.Children, result)
            push(joined.Children, part)
            result = joined
        }
        @ExpectKind(TokenModule.TokenKind::TemplateEnd, "'`' to close template string")
        result.EndToken = @Current
        return result
    }

    << Template text as a quoted string literal: \` and \$ lose their
    << backslash, quotes are escaped and line breaks become \n.
    string TemplateTextLiteral(string raw) {
        string out = "\""
        number i = 0
        while (i < len(raw)) {
            string c = raw[i]
            if (c == "\\" and i + 1 < len(raw)) {
                string next = raw[i + 1]
                if (next == "`" or next == "$") {
                    out = out .: next
                } else {
                    out = out .: c .: next
                }
                i += 2
            } else {
                if (c == "\"") {
                    out = out .: "\\\""
                } else if (c == "\n") {
                    out = out .: "\\n"
                } else if (c != "\r") {
                    out = out .: c
                }
                i += 1
            }
        }
        return out .: "\""
    }

    << func (Type a, Type b) -> Type { body }
    << Children: Parameter nodes, then the body Block. TypeName is the return type.
    Ast.Node ParseLambda() {
        number startToken = @Current
        @ExpectText("func", "'func'")
        @ExpectKind(TokenModule.TokenKind::LeftParen, "'(' after func")
        Ast.Node lambda = @NewNode(Ast.NodeKind::LambdaExpression, "<lambda>", "void", "", startToken, @Current)
        if (not @CheckKind(TokenModule.TokenKind::RightParen)) {
            while (true) {
                number parameterStart = @Current
                string parameterType = @ParseTypeName()
                TokenModule.Token parameterName = @ExpectKind(TokenModule.TokenKind::Identifier, "parameter name")
                push(lambda.Children, @NewNode(
                    Ast.NodeKind::Parameter,
                    TokenModule.GetLexeme(@Source, parameterName),
                    parameterType,
                    "",
                    parameterStart,
                    @Current
                ))
                if (not @MatchKind(TokenModule.TokenKind::Comma)) {
                    break
                }
            }
        }
        @ExpectKind(TokenModule.TokenKind::RightParen, "')' after lambda parameters")
        if (@MatchKind(TokenModule.TokenKind::FunctionArrow)) {
            lambda.TypeName = @ParseTypeName()
        }
        push(lambda.Children, @ParseBlock())
        lambda.EndToken = @Current
        return lambda
    }

    << match (value) { pattern, pattern => body ... _ => body }
    << Children: subject, then MatchArm nodes. An arm's children are its
    << patterns followed by its body Block; `_` is an identifier with Value "wildcard".
    Ast.Node ParseMatch() {
        number startToken = @Current
        @ExpectText("match", "'match'")
        Ast.Node statement = @NewNode(Ast.NodeKind::MatchStatement, "", "", "", startToken, @Current)
        push(statement.Children, @ParseCondition("match"))
        @ExpectKind(TokenModule.TokenKind::LeftBrace, "'{' after match value")
        while (not @CheckKind(TokenModule.TokenKind::RightBrace) and not @IsAtEnd()) {
            if (@MatchKind(TokenModule.TokenKind::Comma) or @MatchKind(TokenModule.TokenKind::Semicolon)) {
                continue
            }
            Ast.Node arm = @NewNode(Ast.NodeKind::MatchArm, "", "", "", @Current, @Current)
            while (true) {
                if (@CheckText("_")) {
                    push(arm.Children, @NewNode(Ast.NodeKind::IdentifierExpression, "_", "", "wildcard", @Current, @Current + 1))
                    @Current += 1
                } else {
                    push(arm.Children, @ParseExpression(3))
                }
                if (not @MatchKind(TokenModule.TokenKind::Comma)) {
                    break
                }
            }
            @ExpectKind(TokenModule.TokenKind::MatchArmOperator, "'=>' after match case")
            if (@CheckKind(TokenModule.TokenKind::LeftBrace)) {
                push(arm.Children, @ParseBlock())
            } else {
                number bodyStart = @Current
                Ast.Node expression = @ParseExpression(0)
                Ast.Node body = @NewNode(Ast.NodeKind::Block, "", "", "", bodyStart, @Current)
                Ast.Node statementNode = @NewNode(Ast.NodeKind::ExpressionStatement, "", "", "", bodyStart, @Current)
                push(statementNode.Children, expression)
                push(body.Children, statementNode)
                push(arm.Children, body)
            }
            arm.EndToken = @Current
            push(statement.Children, arm)
        }
        @ExpectKind(TokenModule.TokenKind::RightBrace, "'}' to close match")
        statement.EndToken = @Current
        return statement
    }

    << try { } catch (error) { }
    << Children: try-block, catch-block, optional binding (VariableDeclaration).
    Ast.Node ParseTry() {
        number startToken = @this.Current;
        @this.ExpectText("try", "'try'");
        Ast.Node statement = @this.NewNode(Ast.NodeKind::TryStatement, "", "", "", startToken, @this.Current);
        push(statement.Children, @this.ParseBlock());
        @this.ExpectText("catch", "'catch' after try block");

        Ast.Node binding = @this.NewNode(Ast.NodeKind::VariableDeclaration, "", "", "", @this.Current, @this.Current);
        bool hasBinding = false;
        if (@this.MatchKind(TokenModule.TokenKind::LeftParen)) {
            TokenModule.Token bindingName = @this.ExpectKind(
                TokenModule.TokenKind::Identifier,
                "error name in catch"
            );
            @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after catch binding");
            binding.Name = TokenModule.GetLexeme(@this.Source, bindingName);
            binding.EndToken = @this.Current;
            statement.Name = binding.Name;
            hasBinding = true;
        }

        push(statement.Children, @this.ParseBlock());
        if (hasBinding) {
            push(statement.Children, binding);
        }
        statement.EndToken = @this.Current;
        return statement;
    }

    << `(Type value)`: a type name directly followed by the start of a value.
    bool LooksLikeTypedExpression() {
        number cursor = @Current + 1
        if (
            cursor >= len(@Tokens)
            or @Tokens[cursor].Kind != TokenModule.TokenKind::Identifier
                and @Tokens[cursor].Kind != TokenModule.TokenKind::Keyword
            or @IsStatementKeyword(@Text(cursor))
        ) {
            return false
        }
        string typeName = @Text(cursor)
        cursor += 1
        while (
            cursor + 1 < len(@Tokens)
            and @Tokens[cursor].Kind == TokenModule.TokenKind::MemberAccessOperator
            and @IsTypeWord(cursor + 1)
        ) {
            cursor += 2
        }
        if (cursor < len(@Tokens) and @Text(cursor) == "<") {
            number depth = 0
            while (cursor < len(@Tokens)) {
                string text = @Text(cursor)
                if (text == "<") {
                    depth += 1
                } else if (text == ">") {
                    depth -= 1
                    if (depth == 0) {
                        cursor += 1
                        break
                    }
                } else if (not @IsTypeWord(cursor) and text != "." and text != ",") {
                    return false
                }
                cursor += 1
            }
        }
        if (cursor >= len(@Tokens)) {
            return false
        }
        TokenModule.Token next = @Tokens[cursor]
        TokenModule.TokenKind kind = next.Kind
        if (
            kind == TokenModule.TokenKind::DecimalIntegerLiteral
            or kind == TokenModule.TokenKind::HexIntegerLiteral
            or kind == TokenModule.TokenKind::BinaryIntegerLiteral
            or kind == TokenModule.TokenKind::FloatLiteral
            or kind == TokenModule.TokenKind::StringLiteral
            or kind == TokenModule.TokenKind::BooleanLiteral
            or kind == TokenModule.TokenKind::NullLiteral
            or kind == TokenModule.TokenKind::Identifier
            or kind == TokenModule.TokenKind::AtSign
        ) {
            return true
        }
        if (kind == TokenModule.TokenKind::LeftParen) {
            << `(len(x) > 1)` is a call: the `(` touches the name.
            TokenModule.Token previous = @Tokens[cursor - 1]
            return next.Start > previous.End
        }
        if (@Text(cursor) == "-" and cursor + 1 < len(@Tokens)) {
            TokenModule.TokenKind after = @Tokens[cursor + 1].Kind
            return find(NUMERIC_TYPES, typeName) >= 0
                and (after == TokenModule.TokenKind::DecimalIntegerLiteral or after == TokenModule.TokenKind::FloatLiteral)
        }
        return false
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

    << 1 assignment (=, +=, ...), 2 ternary ?:, 3 or, 4 and, 5 comparison,
    << 6 range, 7 bitwise, 8 additive/concat, 9 multiplicative,
    << 10 prefix operand, 11 postfix.
    number Precedence(number index) {
        if (@this.Tokens[index].Kind == TokenModule.TokenKind::AssignmentOperator) {
            return 1;
        }
        string op = @this.Text(index);
        if (op == "or") {
            return 3;
        }
        if (op == "and") {
            return 4;
        }
        if (op == "==" or op == "!=" or op == "<" or op == "<=" or op == ">" or op == ">=") {
            return 5;
        }
        if (op == "..") {
            return 6;
        }
        if (op == "shl" or op == "shr" or op == "&" or op == "|" or op == "^") {
            return 7;
        }
        if (op == "+" or op == "-" or op == ".:") {
            return 8;
        }
        if (op == "*" or op == "/" or op == "%") {
            return 9;
        }
        return 0;
    }

    bool OnSameLine() {
        return @this.Tokens[@this.Current].Line == @this.Tokens[@this.Current - 1].Line;
    }

    << `Type { Field = value }` or `Type {}` right after a type path on the same line.
    bool LooksLikeObjectConstruction(Ast.Node left) {
        if (
            not @this.CheckKind(TokenModule.TokenKind::LeftBrace)
            or not @this.OnSameLine()
            or not (
                left.Kind == Ast.NodeKind::IdentifierExpression
                or (left.Kind == Ast.NodeKind::MemberExpression and left.Name != "[]")
            )
        ) {
            return false;
        }
        number next = @this.Current + 1;
        if (next >= len(@this.Tokens)) {
            return false;
        }
        if (@this.Tokens[next].Kind == TokenModule.TokenKind::RightBrace) {
            return true;
        }
        return @this.Tokens[next].Kind == TokenModule.TokenKind::Identifier
            and next + 1 < len(@this.Tokens)
            and @this.Text(next + 1) == "=";
    }

    Ast.Node ParseObjectConstruction(Ast.Node typePath) {
        Ast.Node construction = @this.NewNode(
            Ast.NodeKind::ObjectConstruction,
            "",
            "",
            "",
            typePath.StartToken,
            @this.Current
        );
        push(construction.Children, typePath);
        @this.ExpectKind(TokenModule.TokenKind::LeftBrace, "'{' to start object construction");

        while (
            not @this.CheckKind(TokenModule.TokenKind::RightBrace)
            and not @this.IsAtEnd()
        ) {
            number fieldStart = @this.Current;
            TokenModule.Token fieldName = @this.ExpectKind(
                TokenModule.TokenKind::Identifier,
                "field name in object construction"
            );
            @this.ExpectText("=", "'=' after field name");
            Ast.Node field = @this.NewNode(
                Ast.NodeKind::FieldInitializer,
                TokenModule.GetLexeme(@this.Source, fieldName),
                "",
                "",
                fieldStart,
                @this.Current
            );
            push(field.Children, @this.ParseExpression(2));
            field.EndToken = @this.Current;

            number previous = 1;
            while (previous < len(construction.Children)) {
                if (construction.Children[previous].Name == field.Name) {
                    report("field '" .: field.Name .: "' initialized more than once");
                    break;
                }
                previous += 1;
            }
            push(construction.Children, field);

            if (@this.MatchKind(TokenModule.TokenKind::Comma)) {
                continue;
            }
            if (@this.CheckKind(TokenModule.TokenKind::RightBrace) or @this.IsAtEnd()) {
                break;
            }
            if (not @this.OnSameLine()) {
                continue;
            }
            report(
                "expected ',' or line break after field initializer at "
                    .: @this.Tokens[@this.Current].Line .: ":" .: @this.Tokens[@this.Current].Column
            );
            break;
        }

        @this.ExpectKind(TokenModule.TokenKind::RightBrace, "'}' after object construction");
        construction.EndToken = @this.Current;
        return construction;
    }

    Ast.Node ParseExpression(number minimumPrecedence) {
        Ast.Node left = @this.ParsePrefix();

        while (not @this.IsAtEnd()) {
            if (@this.CheckKind(TokenModule.TokenKind::LeftParen) and @this.OnSameLine()) {
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

            if (@this.MatchKind(TokenModule.TokenKind::MemberAccessOperator)) {
                number startToken = left.StartToken;
                << Keywords are valid member names: Text.join, Fs.read.
                TokenModule.Token member = @Tokens[@Current]
                if (@IsTypeWord(@Current)) {
                    @Current += 1
                } else {
                    member = @ExpectKind(TokenModule.TokenKind::Identifier, "member name after '.'")
                }
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

            if (@this.MatchKind(TokenModule.TokenKind::ScopeResolutionOperator)) {
                number startToken = left.StartToken;
                TokenModule.Token member = @this.Tokens[@this.Current];
                if (not @this.IsTypeWord(@this.Current)) {
                    report(
                        "expected name after '::' at " .: member.Line .: ":" .: member.Column
                    );
                } else {
                    @this.Current += 1;
                }
                Ast.Node scoped = @this.NewNode(
                    Ast.NodeKind::ScopeExpression,
                    TokenModule.GetLexeme(@this.Source, member),
                    "",
                    "",
                    startToken,
                    @this.Current
                );
                push(scoped.Children, left);
                left = scoped;
                continue;
            }

            if (@this.LooksLikeObjectConstruction(left)) {
                left = @this.ParseObjectConstruction(left);
                continue;
            }

            if (@this.CheckKind(TokenModule.TokenKind::LeftBracket) and @this.OnSameLine()) {
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
                and @this.OnSameLine()
                and 11 >= minimumPrecedence
            ) {
                string op = @this.Text(@this.Current);
                number startToken = left.StartToken;
                @this.Current += 1;
                Ast.Node unary = @this.NewNode(
                    Ast.NodeKind::UnaryExpression,
                    op,
                    "postfix",
                    "",
                    startToken,
                    @this.Current
                );
                push(unary.Children, left);
                left = unary;
                continue;
            }

            if (
                @this.CheckKind(TokenModule.TokenKind::ConditionalOperator)
                and 2 >= minimumPrecedence
            ) {
                number startToken = left.StartToken;
                @this.Current += 1;
                Ast.Node whenTrue = @this.ParseExpression(0);
                @this.ExpectKind(TokenModule.TokenKind::Colon, "':' in conditional expression");
                Ast.Node whenFalse = @this.ParseExpression(2);
                Ast.Node conditional = @this.NewNode(
                    Ast.NodeKind::ConditionalExpression,
                    "?",
                    "",
                    "",
                    startToken,
                    @this.Current
                );
                push(conditional.Children, left);
                push(conditional.Children, whenTrue);
                push(conditional.Children, whenFalse);
                left = conditional;
                continue;
            }

            string op = @this.Text(@this.Current);
            number precedence = @this.Precedence(@this.Current);
            if (precedence == 0 or precedence < minimumPrecedence) {
                break;
            }

            bool isAssignment = precedence == 1;
            number startToken = left.StartToken;
            @this.Current += 1;
            number nextMinimum = precedence + 1;
            if (isAssignment) {
                nextMinimum = precedence;
            }
            Ast.Node right = @this.ParseExpression(nextMinimum);
            Ast.NodeKind kind = Ast.NodeKind::BinaryExpression;
            if (isAssignment) {
                kind = Ast.NodeKind::Assignment;
            }
            Ast.Node binary = @this.NewNode(
                kind,
                op,
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
            Ast.Node operand = @this.ParseExpression(10);
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

        if (@MatchKind(TokenModule.TokenKind::AtSign)) {
            Ast.Node self = @NewNode(Ast.NodeKind::ThisExpression, "@this", "", "", startToken, @Current)
            if (@MatchText("this")) {
                self.EndToken = @Current
                return self
            }

            << `@Name` is shorthand for `@this.Name`.
            TokenModule.Token member = @Tokens[@Current]
            if (
                member.Kind != TokenModule.TokenKind::Identifier
                or member.Line != @Tokens[@Current - 1].Line
            ) {
                report("expected 'this' or a member name after '@' at " .: member.Line .: ":" .: member.Column)
                return self
            }
            @Current += 1
            Ast.Node access = @NewNode(
                Ast.NodeKind::MemberExpression,
                TokenModule.GetLexeme(@Source, member),
                "",
                "",
                startToken,
                @Current
            )
            push(access.Children, self)
            return access
        }

        if (@CheckText("func")) {
            return @ParseLambda()
        }

        if (@CheckKind(TokenModule.TokenKind::LeftParen) and @LooksLikeTypedExpression()) {
            << (Type value): the value must have exactly this type.
            @Current += 1
            string typeName = @ParseTypeName()
            Ast.Node typed = @NewNode(Ast.NodeKind::TypedExpression, "", typeName, "", startToken, @Current)
            push(typed.Children, @ParseExpression(0))
            @ExpectKind(TokenModule.TokenKind::RightParen, "')' after typed value")
            typed.EndToken = @Current
            return typed
        }

        if (@this.MatchKind(TokenModule.TokenKind::LeftParen)) {
            Ast.Node expression = @this.ParseExpression(0);
            @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after expression");
            return expression;
        }

        if (@LooksLikeGenericConstruction()) {
            string genericType = @ParseTypeName()
            string baseName = genericType[0 .. Text.indexOf(genericType, "<")]
            return @NewNode(Ast.NodeKind::IdentifierExpression, baseName, genericType, "", startToken, @Current)
        }

        if (@this.LooksLikeTypedCollection()) {
            string typeName = @this.ParseTypeName();
            @this.ExpectKind(TokenModule.TokenKind::LeftParen, "'(' after collection type");
            array<Ast.Node> arguments = array<Ast.Node>()
            if (not @CheckKind(TokenModule.TokenKind::RightParen)) {
                while (true) {
                    push(arguments, @ParseExpression(0))
                    if (not @MatchKind(TokenModule.TokenKind::Comma)) {
                        break
                    }
                }
            }
            @this.ExpectKind(TokenModule.TokenKind::RightParen, "')' after collection initializer");

            << One `{ ... }` literal is the whole collection; anything else is the element list.
            Ast.Node initializer = @CreateEmptyCollection(typeName, @Current)
            if (
                len(arguments) == 1
                and (
                    arguments[0].Kind == Ast.NodeKind::ArrayLiteral
                    or arguments[0].Kind == Ast.NodeKind::DictionaryLiteral
                )
            ) {
                initializer = arguments[0]
            } else if (len(arguments) > 0) {
                if (@IsDictionaryTypeName(typeName)) {
                    report("dictionary constructor takes one { key = value } literal at " .: arguments[0].StartToken)
                }
                initializer = @NewNode(Ast.NodeKind::ArrayLiteral, "", "", "", arguments[0].StartToken, @Current)
                initializer.Children = arguments
            }
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

        if (@CheckKind(TokenModule.TokenKind::TemplateStart)) {
            return @ParseTemplate()
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

    Ast.Node ParseDictionaryKey() {
        if (@this.MatchKind(TokenModule.TokenKind::LeftBracket)) {
            Ast.Node keyNode = @this.ParseExpression(0);
            @this.ExpectKind(TokenModule.TokenKind::RightBracket, "']' after dictionary key");
            return keyNode;
        }
        Ast.Node key = @ParseExpression(3)
        if (key.Kind == Ast.NodeKind::IdentifierExpression) {
            << `{ Name = 1 }`: the key is the text "Name", not the variable Name.
            key.Value = "bare"
        }
        return key
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
                    Ast.Node keyNode = @this.ParseDictionaryKey();
                    @this.ExpectKind(TokenModule.TokenKind::AssignmentOperator, "'=' after dictionary key");
                    Ast.Node value = @this.ParseExpression(0);
                    Ast.Node entry = @this.NewNode(
                        Ast.NodeKind::ExpressionStatement,
                        "entry",
                        "",
                        "",
                        keyNode.StartToken,
                        value.EndToken
                    );
                    push(entry.Children, keyNode);
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
