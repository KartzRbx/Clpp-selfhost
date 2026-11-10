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

TestModuleHeaderWithoutSemicolon();
TestTypedCollectionInitializers();
TestDictionaryEntryKeys();
post("parser core tests passed");
