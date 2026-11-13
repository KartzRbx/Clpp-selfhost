!as SelfHostTests

<<[ The self-hosted parser must parse every file of the compiler and its tests.
    Paths are relative to the repository root: run `clpp-boot test tests` from there. ]>>

link @clpp.fs as Fs
link "../../src/lexer/lexer.clp" as Lexing
link "../../src/parser/parser.clp" as Parsing
link "../../src/ast/ast.clp" as Ast

const array<string> COMPILER_SOURCES = array<string>(
    "src/ast/ast.clp",
    "src/lexer/token.clp",
    "src/lexer/lexer.clp",
    "src/parser/parser.clp",
    "src/checker/naming.clp",
    "src/checker/types.clp"
)

const array<string> TEST_SOURCES = array<string>(
    "tests/lexer/core.clp",
    "tests/parser/core.clp",
    "tests/checker/naming.clp",
    "tests/checker/types.clp",
    "tests/selfhost/parse_sources.clp"
)

Ast.Module ParseFile(string path) {
    guard (Fs.exists(path)) else {
        report("cannot find '" .: path .: "'; run the tests from the repository root")
    }
    string source = Fs.read(path)
    try {
        return Parsing.Parse(source, Lexing.Lex(source))
    } catch (error) {
        report("self-hosted parser rejected " .: path .: ": " .: error)
    }
    return Ast.Module {}
}

bool IsDeclaration(Ast.Node node) {
    return node.Kind == Ast.NodeKind::LinkDeclaration
        or node.Kind == Ast.NodeKind::DefDeclaration
        or node.Kind == Ast.NodeKind::EnumDeclaration
        or node.Kind == Ast.NodeKind::FunctionDeclaration
        or node.Kind == Ast.NodeKind::VariableDeclaration
        or node.Kind == Ast.NodeKind::TypeDeclaration
}

<< Shape checks: a parser can accept a file and still build the wrong tree.
void CheckShape(string path, Ast.Module module, bool allowStatements) {
    if (len(module.Declarations) == 0) {
        report(path .: ": no declarations parsed")
    }
    for (Ast.Node node in module.Declarations) {
        if (not IsDeclaration(node) and not allowStatements) {
            report(path .: ": unexpected top-level " .: node.Kind .: " (a declaration was misparsed)")
        }
        if (node.Kind == Ast.NodeKind::DefDeclaration) {
            for (Ast.Node member in node.Children) {
                if (
                    member.Kind != Ast.NodeKind::FieldDeclaration
                    and member.Kind != Ast.NodeKind::FunctionDeclaration
                ) {
                    report(path .: ": def " .: node.Name .: " has a " .: member.Kind .: " member")
                }
                if (member.Kind == Ast.NodeKind::FunctionDeclaration) {
                    CheckFunction(path, member)
                }
            }
        }
        if (node.Kind == Ast.NodeKind::FunctionDeclaration) {
            CheckFunction(path, node)
        }
    }
}

void CheckFunction(string path, Ast.Node function) {
    Ast.Node body = function.Children[len(function.Children) - 1]
    if (body.Kind != Ast.NodeKind::Block) {
        report(path .: ": function " .: function.Name .: " has no body block")
    }
    number index = 0
    while (index < len(function.Children) - 1) {
        if (function.Children[index].Kind != Ast.NodeKind::Parameter) {
            report(path .: ": function " .: function.Name .: " has a malformed parameter list")
        }
        index += 1
    }
}

Ast.Node FindDeclaration(Ast.Module module, string name) {
    for (Ast.Node node in module.Declarations) {
        if (node.Name == name) {
            return node
        }
    }
    report(module.Name .: ": declaration '" .: name .: "' not found")
    return Ast.Node {}
}

bool HasChild(Ast.Node node, string name) {
    for (Ast.Node child in node.Children) {
        if (child.Name == name) {
            return true
        }
    }
    return false
}

void TestCompilerSourcesParse() {
    for (string path in COMPILER_SOURCES) {
        CheckShape(path, ParseFile(path), false)
    }
}

void TestTestSourcesParse() {
    for (string path in TEST_SOURCES) {
        CheckShape(path, ParseFile(path), true)
    }
}

void TestKnownDeclarations() {
    Ast.Module ast = ParseFile("src/ast/ast.clp")
    if (ast.Name != "") {
        report("ast.clp has no !as header, so its module name must be empty, got " .: ast.Name)
    }
    Ast.Node nodeKind = FindDeclaration(ast, "NodeKind")
    if (
        nodeKind.Kind != Ast.NodeKind::EnumDeclaration
        or not nodeKind.IsExported
        or not HasChild(nodeKind, "EnumMember")
        or len(nodeKind.Children) < 40
    ) {
        report("enum class NodeKind was not parsed with all of its members")
    }

    Ast.Module parser = ParseFile("src/parser/parser.clp")
    Ast.Node parserDef = FindDeclaration(parser, "Parser")
    if (
        parserDef.Kind != Ast.NodeKind::DefDeclaration
        or not HasChild(parserDef, "Tokens")
        or not HasChild(parserDef, "ParseDefBody")
        or not HasChild(parserDef, "ParsePrefix")
    ) {
        report("def Parser was not parsed with its fields and methods")
    }
    if (FindDeclaration(parser, "Parse").Kind != Ast.NodeKind::FunctionDeclaration) {
        report("exported function Parse was not parsed")
    }

    Ast.Module lexer = ParseFile("src/lexer/lexer.clp")
    Ast.Node keywords = FindDeclaration(lexer, "KEYWORDS")
    Ast.Node list = keywords.Children[0].Children[0]
    if (
        not keywords.IsConst
        or keywords.Children[0].Kind != Ast.NodeKind::TypedCollectionExpression
        or list.Kind != Ast.NodeKind::ArrayLiteral
        or len(list.Children) < 70
    ) {
        report("const KEYWORDS = array<string>(...) was not parsed as a multi-element array")
    }
}

TestCompilerSourcesParse()
TestTestSourcesParse()
TestKnownDeclarations()
post("self-hosting parse tests passed")
