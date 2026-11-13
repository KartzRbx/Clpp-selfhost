link "../../src/lexer/lexer.clp" as Lexing;
link "../../src/parser/parser.clp" as Parsing;
link "../../src/ast/ast.clp" as Ast;

Ast.Module ParseSource(string source) {
    return Parsing.Parse(source, Lexing.Lex(source));
}

void ExpectNodeKind(Ast.Node node, Ast.NodeKind kind, string message) {
    if (node.Kind != kind) {
        report(message);
    }
}

void TestModuleHeaderWithoutSemicolon() {
    Ast.Module module = ParseSource("!as Parsing\nauto tabela;");
    if (module.Name != "Parsing" or len(module.Declarations) != 1) {
        report("module header without semicolon was not parsed");
    }
}

void TestOptionalStatementTerminators() {
    string source =
        "!as SemicolonFree\n"
        .: "link \"../../src/lexer/token.clp\" as TokenModule\n"
        .: "number count = 0\n"
        .: "number getValue() {\n"
        .: "    number local = count + 1\n"
        .: "    count = local\n"
        .: "    return local\n"
        .: "}\n"
        .: "void reset() {\n"
        .: "    return\n"
        .: "}\n"
        .: "getValue()\n";
    Ast.Module module = ParseSource(source);

    if (len(module.Declarations) != 5) {
        report("semicolon-free declaration count mismatch");
    }

    if (
        module.Declarations[0].Kind != Ast.NodeKind::LinkDeclaration
        or module.Declarations[1].Kind != Ast.NodeKind::VariableDeclaration
    ) {
        report("semicolon-free link or variable declaration was not parsed");
    }

    Ast.Node getValue = module.Declarations[2];
    if (
        getValue.Kind != Ast.NodeKind::FunctionDeclaration
        or len(getValue.Children) != 1
        or len(getValue.Children[0].Children) != 3
    ) {
        report("semicolon-free function body statements were not parsed");
    }

    Ast.Node reset = module.Declarations[3];
    if (
        reset.Kind != Ast.NodeKind::FunctionDeclaration
        or len(reset.Children[0].Children) != 1
        or reset.Children[0].Children[0].Kind != Ast.NodeKind::ReturnStatement
    ) {
        report("semicolon-free empty return was not parsed");
    }

    if (module.Declarations[4].Kind != Ast.NodeKind::ExpressionStatement) {
        report("semicolon-free expression statement was not parsed");
    }
}

void TestStructuralTypeDeclarations() {
    string source =
        "!as Types\n"
        .: "def Sla = Vector2 & {\n"
        .: "    Sla: string & {\n"
        .: "        Child: number\n"
        .: "    }\n"
        .: "}\n"
        .: "def Position = typeof(Vector2)\n";
    Ast.Module module = ParseSource(source);

    if (len(module.Declarations) != 2) {
        report("type declaration count mismatch");
    }

    Ast.Node sla = module.Declarations[0];
    if (
        sla.Kind != Ast.NodeKind::TypeDeclaration
        or sla.Name != "Sla"
        or len(sla.Children) != 1
    ) {
        report("named type declaration was not parsed");
    }

    Ast.Node rootType = sla.Children[0];
    if (
        rootType.Kind != Ast.NodeKind::TypeIntersection
        or rootType.Children[0].Kind != Ast.NodeKind::TypeReference
        or rootType.Children[0].TypeName != "Vector2"
        or rootType.Children[1].Kind != Ast.NodeKind::ObjectType
    ) {
        report("type intersection with an object type was not parsed");
    }

    Ast.Node field = rootType.Children[1].Children[0];
    if (field.Kind != Ast.NodeKind::TypeField or field.Name != "Sla") {
        report("object type field was not parsed");
    }
    Ast.Node nested = field.Children[0];
    if (
        nested.Kind != Ast.NodeKind::TypeIntersection
        or nested.Children[0].TypeName != "string"
        or nested.Children[1].Kind != Ast.NodeKind::ObjectType
        or nested.Children[1].Children[0].Name != "Child"
        or nested.Children[1].Children[0].Children[0].TypeName != "number"
    ) {
        report("nested structural field type was not parsed");
    }

    Ast.Node position = module.Declarations[1];
    if (
        position.Kind != Ast.NodeKind::TypeDeclaration
        or position.Children[0].Kind != Ast.NodeKind::TypeofType
        or position.Children[0].Children[0].Name != "Vector2"
    ) {
        report("typeof type expression was not parsed");
    }
}

void TestTypedCollectionInitializers() {
    string source =
        "!as Collections;\n"
        .: "auto tabela;\n"
        .: "tabela = dictionary<string, number>({});\n"
        .: "dictionary<string, number> tabelaVazia;\n"
        .: "dictionary<string, number> tabelaLiteral = {};\n"
        .: "auto inicializada = dictionary<string, number>({});\n"
        .: "array<number> itens;\n";
    Ast.Module module = ParseSource(source);

    if (len(module.Declarations) != 6) {
        report("typed collection declaration count mismatch");
    }

    Ast.Node autoDeclaration = module.Declarations[0];
    ExpectNodeKind(
        autoDeclaration,
        Ast.NodeKind::VariableDeclaration,
        "auto collection declaration was not parsed"
    );
    if (not autoDeclaration.IsAuto or len(autoDeclaration.Children) != 0) {
        report("uninitialized auto declaration must not guess a collection type");
    }

    Ast.Node assignmentStatement = module.Declarations[1];
    ExpectNodeKind(
        assignmentStatement,
        Ast.NodeKind::ExpressionStatement,
        "typed collection assignment was not parsed as a statement"
    );
    Ast.Node assignment = assignmentStatement.Children[0];
    ExpectNodeKind(assignment, Ast.NodeKind::Assignment, "collection assignment node missing");
    Ast.Node typedAssignment = assignment.Children[1];
    ExpectNodeKind(
        typedAssignment,
        Ast.NodeKind::TypedCollectionExpression,
        "typed dictionary construction node missing"
    );
    ExpectNodeKind(
        typedAssignment.Children[0],
        Ast.NodeKind::DictionaryLiteral,
        "empty typed dictionary constructor must contain a dictionary literal"
    );

    Ast.Node uninitializedDictionary = module.Declarations[2];
    if (
        len(uninitializedDictionary.Children) != 1
        or uninitializedDictionary.TypeName != "dictionary<string,number>"
    ) {
        report("uninitialized typed dictionary must receive an empty initializer");
    }
    ExpectNodeKind(
        uninitializedDictionary.Children[0],
        Ast.NodeKind::DictionaryLiteral,
        "uninitialized typed dictionary must initialize as a dictionary"
    );

    Ast.Node literalDictionary = module.Declarations[3];
    ExpectNodeKind(
        literalDictionary.Children[0],
        Ast.NodeKind::DictionaryLiteral,
        "empty literal in a dictionary declaration must use dictionary context"
    );

    Ast.Node initializedAuto = module.Declarations[4];
    Ast.Node typedInitializer = initializedAuto.Children[0];
    ExpectNodeKind(
        typedInitializer,
        Ast.NodeKind::TypedCollectionExpression,
        "auto initialized from typed dictionary must retain the type constructor"
    );
    ExpectNodeKind(
        typedInitializer.Children[0],
        Ast.NodeKind::DictionaryLiteral,
        "typed empty dictionary initializer was parsed as an array"
    );

    Ast.Node emptyArray = module.Declarations[5];
    if (len(emptyArray.Children) != 1) {
        report("uninitialized typed array must receive an empty initializer");
    }
    ExpectNodeKind(
        emptyArray.Children[0],
        Ast.NodeKind::ArrayLiteral,
        "uninitialized typed array must initialize as an array"
    );
}

void TestDictionaryEntryKeys() {
    string source =
        "!as Collections;\n"
        .: "auto tabela = dictionary<string, number>({\n"
        .: "    Teste = 10,\n"
        .: "    \"Teste2\" = 10,\n"
        .: "    [\"Testesla\"] = 100\n"
        .: "});\n";
    Ast.Module module = ParseSource(source);

    if (len(module.Declarations) != 1) {
        report("dictionary literal declaration count mismatch");
    }

    Ast.Node typed = module.Declarations[0].Children[0];
    ExpectNodeKind(
        typed,
        Ast.NodeKind::TypedCollectionExpression,
        "dictionary literal must be wrapped in its typed constructor"
    );
    Ast.Node dictionary = typed.Children[0];
    ExpectNodeKind(
        dictionary,
        Ast.NodeKind::DictionaryLiteral,
        "keyed collection entries must be parsed as a dictionary"
    );
    if (len(dictionary.Children) != 3) {
        report("dictionary entry count mismatch");
    }

    Ast.Node identifierEntry = dictionary.Children[0];
    if (
        identifierEntry.Name != "entry"
        or identifierEntry.Children[0].Kind != Ast.NodeKind::IdentifierExpression
        or identifierEntry.Children[0].Name != "Teste"
    ) {
        report("bare identifier dictionary key was not parsed correctly");
    }

    Ast.Node stringEntry = dictionary.Children[1];
    if (
        stringEntry.Children[0].Kind != Ast.NodeKind::LiteralExpression
        or stringEntry.Children[0].Value != "\"Teste2\""
    ) {
        report("quoted dictionary key was not parsed correctly");
    }

    Ast.Node computedEntry = dictionary.Children[2];
    if (
        computedEntry.Children[0].Kind != Ast.NodeKind::LiteralExpression
        or computedEntry.Children[0].Value != "\"Testesla\""
    ) {
        report("bracketed dictionary key was not parsed correctly");
    }
}

void TestControlFlowStatements() {
    string source =
        "!as ControlFlow\n"
        .: "void Run(array<number> values) {\n"
        .: "    number index = 0\n"
        .: "    if (index == 0) {\n"
        .: "        index = 1\n"
        .: "    } else if (index == 1) {\n"
        .: "        index = 2\n"
        .: "    } else {\n"
        .: "        index = 3\n"
        .: "    }\n"
        .: "    while (index < 10) {\n"
        .: "        if (index == 5) {\n"
        .: "            break\n"
        .: "        }\n"
        .: "        index += 1\n"
        .: "        continue\n"
        .: "    }\n"
        .: "    do {\n"
        .: "        index -= 1\n"
        .: "    } while (index > 0)\n"
        .: "    for (number value in values) {\n"
        .: "        post(value)\n"
        .: "    }\n"
        .: "    guard (len(values) > 0) else {\n"
        .: "        return\n"
        .: "    }\n"
        .: "    try {\n"
        .: "        report(\"falhou\")\n"
        .: "    } catch (error) {\n"
        .: "        warn(error)\n"
        .: "    }\n"
        .: "}\n"
    Ast.Module module = ParseSource(source)

    if (len(module.Declarations) != 1) {
        report("control flow function declaration count mismatch")
        return
    }
    Ast.Node body = module.Declarations[0].Children[1]
    if (len(body.Children) != 7) {
        report("control flow statement count mismatch")
        return
    }

    Ast.Node ifStatement = body.Children[1]
    ExpectNodeKind(ifStatement, Ast.NodeKind::IfStatement, "if statement was not parsed")
    if (
        len(ifStatement.Children) != 3
        or ifStatement.Children[0].Kind != Ast.NodeKind::BinaryExpression
        or ifStatement.Children[2].Kind != Ast.NodeKind::IfStatement
        or len(ifStatement.Children[2].Children) != 3
        or ifStatement.Children[2].Children[2].Kind != Ast.NodeKind::Block
    ) {
        report("else if / else chain was not parsed")
    }

    Ast.Node whileStatement = body.Children[2]
    ExpectNodeKind(whileStatement, Ast.NodeKind::WhileStatement, "while statement was not parsed")
    Ast.Node whileBody = whileStatement.Children[1]
    if (
        len(whileBody.Children) != 3
        or whileBody.Children[0].Children[1].Children[0].Kind != Ast.NodeKind::BreakStatement
        or whileBody.Children[1].Children[0].Kind != Ast.NodeKind::Assignment
        or whileBody.Children[1].Children[0].Name != "+="
        or whileBody.Children[2].Kind != Ast.NodeKind::ContinueStatement
    ) {
        report("break without semicolon must not swallow the next line as a declaration")
    }

    Ast.Node doWhile = body.Children[3]
    ExpectNodeKind(doWhile, Ast.NodeKind::DoWhileStatement, "do-while statement was not parsed")
    if (
        doWhile.Children[0].Kind != Ast.NodeKind::Block
        or doWhile.Children[1].Kind != Ast.NodeKind::BinaryExpression
    ) {
        report("do-while children must be body then condition")
    }

    Ast.Node forStatement = body.Children[4]
    ExpectNodeKind(forStatement, Ast.NodeKind::ForInStatement, "for-in statement was not parsed")
    if (
        forStatement.Children[0].Kind != Ast.NodeKind::VariableDeclaration
        or forStatement.Children[0].Name != "value"
        or forStatement.Children[0].TypeName != "number"
        or forStatement.Children[1].Name != "values"
    ) {
        report("for-in loop variable or iterable was not parsed")
    }

    Ast.Node guardStatement = body.Children[5]
    ExpectNodeKind(guardStatement, Ast.NodeKind::GuardStatement, "guard statement was not parsed")
    if (guardStatement.Children[1].Children[0].Kind != Ast.NodeKind::ReturnStatement) {
        report("guard else block was not parsed")
    }

    Ast.Node tryStatement = body.Children[6]
    ExpectNodeKind(tryStatement, Ast.NodeKind::TryStatement, "try statement was not parsed")
    if (
        tryStatement.Name != "error"
        or len(tryStatement.Children) != 3
        or tryStatement.Children[2].Kind != Ast.NodeKind::VariableDeclaration
    ) {
        report("catch binding was not parsed")
    }
}

void TestSelfHostingExpressions() {
    string source =
        "!as Expressions\n"
        .: "const number LIMIT = 3\n"
        .: "Ast.Node node = Ast.Node {\n"
        .: "    Kind = Ast.NodeKind::Module,\n"
        .: "    Name = \"raiz\"\n"
        .: "    IsAuto = false\n"
        .: "}\n"
        .: "auto vazio = Checker {}\n"
        .: "auto kind = isDictionary ? Ast.NodeKind::DictionaryLiteral : Ast.NodeKind::ArrayLiteral\n"
        .: "@this.Current += 1\n"
        .: "auto ativo = not @this.IsAtEnd() and count < LIMIT\n"
    Ast.Module module = ParseSource(source)

    if (len(module.Declarations) != 6) {
        report("self-hosting expression declaration count mismatch")
        return
    }

    Ast.Node limit = module.Declarations[0]
    if (not limit.IsConst or limit.TypeName != "number" or len(limit.Children) != 1) {
        report("const declaration was not parsed")
    }

    Ast.Node node = module.Declarations[1]
    if (node.Kind != Ast.NodeKind::VariableDeclaration or node.TypeName != "Ast.Node") {
        report("qualified type declaration was not parsed")
    }
    Ast.Node construction = node.Children[0]
    ExpectNodeKind(construction, Ast.NodeKind::ObjectConstruction, "object construction was not parsed")
    if (
        len(construction.Children) != 4
        or construction.Children[0].Kind != Ast.NodeKind::MemberExpression
        or construction.Children[1].Kind != Ast.NodeKind::FieldInitializer
        or construction.Children[1].Name != "Kind"
        or construction.Children[1].Children[0].Kind != Ast.NodeKind::ScopeExpression
        or construction.Children[1].Children[0].Name != "Module"
        or construction.Children[3].Name != "IsAuto"
    ) {
        report("object fields separated by commas or line breaks were not parsed")
    }

    Ast.Node empty = module.Declarations[2].Children[0]
    if (empty.Kind != Ast.NodeKind::ObjectConstruction or len(empty.Children) != 1) {
        report("empty object construction was not parsed")
    }

    Ast.Node conditional = module.Declarations[3].Children[0]
    if (
        conditional.Kind != Ast.NodeKind::ConditionalExpression
        or len(conditional.Children) != 3
        or conditional.Children[1].Name != "DictionaryLiteral"
        or conditional.Children[2].Name != "ArrayLiteral"
    ) {
        report("conditional expression was not parsed")
    }

    Ast.Node increment = module.Declarations[4].Children[0]
    if (
        increment.Kind != Ast.NodeKind::Assignment
        or increment.Name != "+="
        or increment.Children[0].Children[0].Kind != Ast.NodeKind::ThisExpression
    ) {
        report("compound assignment on @this was not parsed")
    }

    Ast.Node logical = module.Declarations[5].Children[0]
    if (
        logical.Kind != Ast.NodeKind::BinaryExpression
        or logical.Name != "and"
        or logical.Children[0].Kind != Ast.NodeKind::UnaryExpression
        or logical.Children[1].Name != "<"
    ) {
        report("prefix 'not' must bind tighter than 'and' and comparisons")
    }
}

void TestAtMemberShorthand() {
    Ast.Module module = ParseSource("!as Shorthand\n@Current += 1\n@Advance()\nauto self = @this\n")

    if (len(module.Declarations) != 3) {
        report("@ shorthand declaration count mismatch")
        return
    }

    Ast.Node increment = module.Declarations[0].Children[0]
    Ast.Node target = increment.Children[0]
    if (
        increment.Kind != Ast.NodeKind::Assignment
        or target.Kind != Ast.NodeKind::MemberExpression
        or target.Name != "Current"
        or target.Children[0].Kind != Ast.NodeKind::ThisExpression
    ) {
        report("@Current must parse as @this.Current")
    }

    Ast.Node call = module.Declarations[1].Children[0]
    if (
        call.Kind != Ast.NodeKind::CallExpression
        or call.Children[0].Kind != Ast.NodeKind::MemberExpression
        or call.Children[0].Name != "Advance"
    ) {
        report("@Advance() must parse as a call to @this.Advance")
    }

    if (module.Declarations[2].Children[0].Kind != Ast.NodeKind::ThisExpression) {
        report("plain @this must still parse")
    }
}

void TestDocComments() {
    string source =
        "!as Docs\n"
        .: "<<{\n"
        .: "    !name = \"Distance\"\n"
        .: "    !desc = \"Distance between two points\"\n"
        .: "    !params = \"a: number, b: number\"\n"
        .: "    !return = \"number\"\n"
        .: "}>>\n"
        .: "export number Distance(number a, number b) {\n"
        .: "    return b - a\n"
        .: "}\n"
        .: "<<{ !desc = \"Maximum speed\" }>>\n"
        .: "const number MAX_SPEED = 10\n"
    Ast.Module module = ParseSource(source)

    if (len(module.Declarations) != 2) {
        report("documented declaration count mismatch")
        return
    }

    Ast.Node distance = module.Declarations[0]
    if (
        distance.Kind != Ast.NodeKind::FunctionDeclaration
        or len(distance.Docs) != 4
        or distance.Docs[0].Name != "name"
        or distance.Docs[0].Value != "Distance"
        or distance.Docs[1].Value != "Distance between two points"
        or distance.Docs[3].Name != "return"
    ) {
        report("function doc comment tags were not attached")
    }

    Ast.Node speed = module.Declarations[1]
    if (len(speed.Docs) != 1 or speed.Docs[0].Value != "Maximum speed") {
        report("variable doc comment was not attached")
    }
}

bool ParseFails(string source) {
    try {
        ParseSource(source)
    } catch (error) {
        return true
    }
    return false
}

void TestInvalidDocComments() {
    if (not ParseFails("!as Bad\n<<{ !name = \"Other\" }>>\nnumber Real() {\n    return 1\n}\n")) {
        report("a !name that differs from the declaration must be rejected")
    }
    if (not ParseFails("!as Bad\n<<{ !author = \"me\" }>>\nnumber x = 1\n")) {
        report("unknown doc tags must be rejected")
    }
    if (not ParseFails("!as Bad\n<<{ !desc = \"a\"\n !desc = \"b\" }>>\nnumber x = 1\n")) {
        report("duplicate doc tags must be rejected")
    }
    if (not ParseFails("!as Bad\n<<{ !desc = \"orphan\" }>>\npost(1)\n")) {
        report("a doc comment above a statement must be rejected")
    }
}

void TestModuleHeaderIsOptional() {
    Ast.Module module = ParseSource("number count = 1\npost(count)\n")
    if (module.Name != "" or len(module.Declarations) != 2) {
        report("a file without !as must parse with an empty module name")
    }
}

TestModuleHeaderWithoutSemicolon();
TestOptionalStatementTerminators();
TestStructuralTypeDeclarations();
TestTypedCollectionInitializers();
TestDictionaryEntryKeys();
TestControlFlowStatements()
TestSelfHostingExpressions()
TestAtMemberShorthand()
TestDocComments()
TestModuleHeaderIsOptional()
TestInvalidDocComments()
post("parser core tests passed");
