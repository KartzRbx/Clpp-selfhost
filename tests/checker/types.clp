link "../../src/lexer/lexer.clp" as Lexing;
link "../../src/parser/parser.clp" as Parsing;
link "../../src/checker/types.clp" as TypeChecker;

void TestValidStructuralTypes() {
    string source =
        "!as Types\n"
        .: "def Sla = Vector2 & {\n"
        .: "    Sla: string & {\n"
        .: "        Child: number\n"
        .: "    }\n"
        .: "}\n";
    TypeChecker.CheckTypeDeclarations(Parsing.Parse(source, Lexing.Lex(source)));
}

void TestDuplicateTypeFieldsAreRejected() {
    string source =
        "!as DuplicateFields\n"
        .: "def Item = { Name: string, Name: number }\n";
    bool caughtError = false;
    try {
        TypeChecker.CheckTypeDeclarations(Parsing.Parse(source, Lexing.Lex(source)));
    } catch (error) {
        caughtError = true;
    }

    if (not caughtError) {
        report("duplicate structural type fields must be rejected");
    }
}

void TestDuplicateTypeDeclarationsAreRejected() {
    string source =
        "!as DuplicateTypes\n"
        .: "def Item = string\n"
        .: "def Item = number\n";
    bool caughtError = false;
    try {
        TypeChecker.CheckTypeDeclarations(Parsing.Parse(source, Lexing.Lex(source)));
    } catch (error) {
        caughtError = true;
    }

    if (not caughtError) {
        report("duplicate type declarations must be rejected");
    }
}

TestValidStructuralTypes();
TestDuplicateTypeFieldsAreRejected();
TestDuplicateTypeDeclarationsAreRejected();
post("type checker tests passed");
