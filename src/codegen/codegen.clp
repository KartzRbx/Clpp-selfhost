<<[ Code generator: turns the AST of one module into CL++ bytecode (.clpb text)
    for the register virtual machine clpp-vm.

    Registers: parameters first (methods: r0 is @this), then locals, then
    temporaries. Temporaries are released at the end of every statement.
    Top-level variables become globals; top-level statements run in __main.

    Linked files are compiled into the same output. Their names get a module
    prefix (`ast$Node`), and their top-level code runs in `<prefix>$__init`,
    dependencies first, before __main. ]>>

link "../ast/ast.clp" as Ast
link "../lexer/token.clp" as TokenModule
link "../lexer/lexer.clp" as Lexing
link "../parser/parser.clp" as Parsing
link @clpp.text as Text
link @clpp.fs as Fs

const array<string> NUMERIC_TYPES = array<string>(
    "number", "float", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f16", "f32", "f64"
)

const array<string> BUILTINS = array<string>(
    "post", "warn", "report", "len", "push", "pop", "insert", "remove", "find", "sort", "args"
)

def Scope {
    array<string> Names
    array<number> Registers
    array<string> Types
}

def Loop {
    string ContinueLabel
    string BreakLabel
    number TryDepth
}

<< Modules already compiled into the output, shared by all generators.
def Registry {
    array<string> Paths
    array<string> Prefixes
    array<Generator> Modules
    array<string> Inits
}

export def Generator {
    string Prefix
    Registry Modules
    array<string> LinkAliases
    array<string> LinkPrefixes
    string SourcePath
    array<TokenModule.Token> Tokens
    array<string> Lines
    array<string> Body
    array<string> Functions
    array<Ast.Node> Defs
    array<string> Enums
    array<string> Globals
    array<string> GlobalTypes
    array<string> NativeAliases
    array<string> NativeModules
    array<Scope> Scopes
    array<Loop> Loops
    number Next
    number MaxRegisters
    number LabelCount
    number LambdaCount
    number TryDepth
    number LastLine
    bool InMethod
    bool InMain

    << ------------------------------------------------------------ output

    void Emit(string instruction) {
        push(@Body, "  " .: instruction)
    }

    string NewLabel() {
        @LabelCount += 1
        return "L" .: @LabelCount
    }

    void Mark(string label) {
        push(@Body, label .: ":")
    }

    void Fail(Ast.Node node, string message) {
        number line = 0
        if (node.StartToken < len(@Tokens)) {
            line = @Tokens[node.StartToken].Line
        }
        report("codegen: " .: message .: " (" .: @SourcePath .: " line " .: line .: ")")
    }

    << ------------------------------------------------------------ modules

    << Name as it appears in the bytecode: `prefix$name` in linked modules.
    string Q(string name) {
        if (@Prefix == "") {
            return name
        }
        return @Prefix .: "$" .: name
    }

    << Prefix of a linked file alias, or "".
    string LinkedPrefix(Ast.Node node) {
        if (node.Kind != Ast.NodeKind::IdentifierExpression or @Local(node.Name) >= 0) {
            return ""
        }
        number index = find(@LinkAliases, node.Name)
        if (index < 0) {
            return ""
        }
        return @LinkPrefixes[index]
    }

    << Bytecode name of a def written `Name` or `Alias.Name` in this module.
    string DefName(string written) {
        number dot = Text.indexOf(written, ".")
        if (dot > 0) {
            number alias = find(@LinkAliases, written[0 .. dot])
            if (alias >= 0) {
                return @LinkPrefixes[alias] .: "$" .: written[dot + 1 .. len(written)]
            }
        }
        return @Q(written)
    }

    << The def node and the generator (module) that declares it.
    Ast.Node DefNode(string written) {
        number dot = Text.indexOf(written, ".")
        if (dot > 0) {
            number alias = find(@LinkAliases, written[0 .. dot])
            if (alias >= 0) {
                return @LinkedModule(@LinkPrefixes[alias]).FindDef(written[dot + 1 .. len(written)])
            }
        }
        return @FindDef(written)
    }

    Generator DefOwner(string written) {
        number dot = Text.indexOf(written, ".")
        if (dot > 0) {
            number alias = find(@LinkAliases, written[0 .. dot])
            if (alias >= 0) {
                return @LinkedModule(@LinkPrefixes[alias])
            }
        }
        return @this
    }

    << Fields of a def including inherited ones, base fields first.
    array<Ast.Node> AllFields(Ast.Node definition) {
        array<Ast.Node> fields = array<Ast.Node>()
        if (definition.Value != "") {
            Generator owner = @DefOwner(definition.Value)
            Ast.Node base = @DefNode(definition.Value)
            if (base.Name == "") {
                @Fail(definition, "base def '" .: definition.Value .: "' not found")
            }
            for (Ast.Node inherited in owner.AllFields(base)) {
                bool overridden = false
                for (Ast.Node member in definition.Children) {
                    if (member.Kind == Ast.NodeKind::FieldDeclaration and member.Name == inherited.Name) {
                        overridden = true
                    }
                }
                if (not overridden) {
                    push(fields, inherited)
                }
            }
        }
        for (Ast.Node member in definition.Children) {
            if (member.Kind == Ast.NodeKind::FieldDeclaration) {
                push(fields, member)
            }
        }
        return fields
    }

    Generator LinkedModule(string prefix) {
        return @Modules.Modules[find(@Modules.Prefixes, prefix)]
    }

    << Folder part of a path ("" when there is none).
    string Directory(string path) {
        string clean = Text.replace(path, "\\", "/")
        number slash = Text.lastIndexOf(clean, "/")
        if (slash < 0) {
            return ""
        }
        return clean[0 .. slash]
    }

    << Joins a link path to the folder of the current file, resolving . and ..
    string Resolve(string relative) {
        string clean = Text.replace(relative, "\\", "/")
        array<string> parts = array<string>()
        bool absolute = Text.startsWith(clean, "/") or Text.contains(clean, ":")
        if (not absolute and @Directory(@SourcePath) != "") {
            parts = Text.split(@Directory(@SourcePath), "/")
        }
        array<string> result = array<string>()
        for (string part in parts) {
            push(result, part)
        }
        for (string part in Text.split(clean, "/")) {
            if (part == "..") {
                if (len(result) > 0 and result[len(result) - 1] != "..") {
                    pop(result)
                } else {
                    push(result, part)
                }
            } else if (part != "." and part != "") {
                push(result, part)
            }
        }
        string joined = Text.join(result, "/")
        if (Text.startsWith(clean, "/")) {
            joined = "/" .: joined
        }
        return joined
    }

    << Compiles a linked file once and returns its prefix.
    string Link(Ast.Node declaration) {
        string quoted = declaration.Value
        string path = @Resolve(quoted[1 .. len(quoted) - 1])
        number known = find(@Modules.Paths, path)
        if (known >= 0) {
            return @Modules.Prefixes[known]
        }
        guard (Fs.exists(path)) else {
            @Fail(declaration, "linked file not found: " .: path)
        }

        string stem = path
        if (Text.lastIndexOf(stem, "/") >= 0) {
            stem = stem[Text.lastIndexOf(stem, "/") + 1 .. len(stem)]
        }
        if (Text.endsWith(stem, ".clp")) {
            stem = stem[0 .. len(stem) - 4]
        }
        string prefix = stem
        number suffix = 2
        while (find(@Modules.Prefixes, prefix) >= 0) {
            prefix = stem .: suffix
            suffix += 1
        }

        string source = Fs.read(path)
        auto tokens = Lexing.Lex(source)
        Generator child = Generator {
            Prefix = prefix,
            Modules = @Modules,
            LinkAliases = array<string>(),
            LinkPrefixes = array<string>(),
            SourcePath = path,
            Tokens = tokens,
            Lines = @Lines,
            Body = array<string>(),
            Functions = array<string>(),
            Defs = array<Ast.Node>(),
            Enums = array<string>(),
            Globals = array<string>(),
            GlobalTypes = array<string>(),
            NativeAliases = array<string>(),
            NativeModules = array<string>(),
            Scopes = array<Scope>(),
            Loops = array<Loop>()
        }
        push(@Modules.Paths, path)
        push(@Modules.Prefixes, prefix)
        push(@Modules.Modules, child)
        child.Generate(Parsing.Parse(source, tokens))
        return prefix
    }

    void TrackLine(Ast.Node node) {
        if (node.StartToken < len(@Tokens)) {
            number line = @Tokens[node.StartToken].Line
            if (line != @LastLine) {
                push(@Body, ".line " .: line)
                @LastLine = line
            }
        }
    }

    << ------------------------------------------------------------ registers

    number Alloc() {
        number register = @Next
        @Next += 1
        if (@Next > @MaxRegisters) {
            @MaxRegisters = @Next
        }
        return register
    }

    void PushScope() {
        push(@Scopes, Scope { Names = array<string>(), Registers = array<number>(), Types = array<string>() })
    }

    void PopScope() {
        pop(@Scopes)
    }

    void Declare(string name, number register) {
        @DeclareTyped(name, register, "")
    }

    void DeclareTyped(string name, number register, string typeName) {
        Scope scope = @Scopes[len(@Scopes) - 1]
        push(scope.Names, name)
        push(scope.Registers, register)
        push(scope.Types, typeName)
    }

    << Declared numeric type of a variable, or "".
    string NumericType(string name) {
        number depth = len(@Scopes) - 1
        while (depth >= 0) {
            Scope scope = @Scopes[depth]
            number index = len(scope.Names) - 1
            while (index >= 0) {
                if (scope.Names[index] == name) {
                    return scope.Types[index]
                }
                index -= 1
            }
            depth -= 1
        }
        number global = find(@Globals, name)
        if (global >= 0) {
            return @GlobalTypes[global]
        }
        return ""
    }

    bool IsLiteralNumber(Ast.Node node) {
        if (node.Kind == Ast.NodeKind::UnaryExpression and node.Name == "-") {
            return @IsLiteralNumber(node.Children[0])
        }
        if (node.Kind != Ast.NodeKind::LiteralExpression or len(node.Value) == 0) {
            return false
        }
        string first = node.Value[0]
        return first != "\"" and first != "'" and node.Value != "true" and node.Value != "false" and node.Value != "null"
    }

    void Convert(number register, string typeName, bool exact) {
        string flag = "0"
        if (exact) {
            flag = "1"
        }
        @Emit("CONV r" .: register .: " r" .: register .: " " .: typeName .: " " .: flag)
    }

    << Module key ("math") for a native alias ("Math"), or "".
    string NativeModule(Ast.Node node) {
        if (node.Kind != Ast.NodeKind::IdentifierExpression or @Local(node.Name) >= 0) {
            return ""
        }
        number index = find(@NativeAliases, node.Name)
        if (index < 0) {
            return ""
        }
        return @NativeModules[index]
    }

    << Register of a local variable, or -1.
    number Local(string name) {
        number depth = len(@Scopes) - 1
        while (depth >= 0) {
            Scope scope = @Scopes[depth]
            number index = len(scope.Names) - 1
            while (index >= 0) {
                if (scope.Names[index] == name) {
                    return scope.Registers[index]
                }
                index -= 1
            }
            depth -= 1
        }
        return -1
    }

    bool IsGlobal(string name) {
        return @Local(name) < 0 and find(@Globals, name) >= 0
    }

    Ast.Node FindDef(string name) {
        for (Ast.Node definition in @Defs) {
            if (definition.Name == name) {
                return definition
            }
        }
        return Ast.Node { Name = "" }
    }

    << ------------------------------------------------------------ module

    string Generate(Ast.Module module) {
        array<Ast.Node> topLevel = array<Ast.Node>()
        for (Ast.Node declaration in module.Declarations) {
            if (declaration.Kind == Ast.NodeKind::FunctionDeclaration) {
                push(@Functions, declaration.Name)
            } else if (declaration.Kind == Ast.NodeKind::DefDeclaration) {
                push(@Defs, declaration)
            } else if (declaration.Kind == Ast.NodeKind::EnumDeclaration) {
                push(@Enums, declaration.Name)
            } else if (declaration.Kind == Ast.NodeKind::VariableDeclaration) {
                push(@Globals, declaration.Name)
                if (find(NUMERIC_TYPES, declaration.TypeName) >= 0) {
                    push(@GlobalTypes, declaration.TypeName)
                } else {
                    push(@GlobalTypes, "")
                }
                push(topLevel, declaration)
            } else if (declaration.Kind == Ast.NodeKind::LinkDeclaration) {
                if (len(declaration.Value) > 6 and declaration.Value[0 .. 6] == "@clpp.") {
                    push(@NativeAliases, declaration.Name)
                    push(@NativeModules, declaration.Value[6 .. len(declaration.Value)])
                } else {
                    push(@LinkAliases, declaration.Name)
                    push(@LinkPrefixes, @Link(declaration))
                }
            } else if (declaration.Kind != Ast.NodeKind::TypeDeclaration) {
                push(topLevel, declaration)
            }
        }

        if (@Prefix == "") {
            << The entry module writes the header first: linked files were
            << appended while their links were read, so put it in front.
            array<string> linked = array<string>()
            for (string line in @Lines) {
                push(linked, line)
            }
            while (len(@Lines) > 0) {
                pop(@Lines)
            }
            push(@Lines, ".clpb 1")
            string moduleName = module.Name
            if (moduleName == "") {
                moduleName = "Main"
            }
            push(@Lines, ".module " .: moduleName)
            for (string line in linked) {
                push(@Lines, line)
            }
        }
        if (@SourcePath != "") {
            push(@Lines, ".source " .: @SourcePath)
        }
        for (Ast.Node declaration in module.Declarations) {
            if (declaration.Kind == Ast.NodeKind::EnumDeclaration) {
                string line = ".enum " .: @Q(declaration.Name)
                for (Ast.Node member in declaration.Children) {
                    line = line .: " " .: member.Name
                }
                push(@Lines, line)
            }
        }
        for (Ast.Node definition in @Defs) {
            string line = ".def " .: @Q(definition.Name)
            for (Ast.Node member in @AllFields(definition)) {
                line = line .: " " .: member.Name
                if (find(NUMERIC_TYPES, member.TypeName) >= 0) {
                    line = line .: ":" .: member.TypeName
                }
            }
            push(@Lines, line)
            if (definition.Value != "") {
                push(@Lines, ".base " .: @Q(definition.Name) .: " " .: @DefName(definition.Value))
            }
        }
        for (string global in @Globals) {
            push(@Lines, ".global " .: @Q(global))
        }

        for (Ast.Node definition in @Defs) {
            @Defaults(definition)
            for (Ast.Node member in definition.Children) {
                if (member.Kind == Ast.NodeKind::FunctionDeclaration) {
                    @Function(member, definition.Name)
                }
            }
        }
        for (Ast.Node declaration in module.Declarations) {
            if (declaration.Kind == Ast.NodeKind::FunctionDeclaration) {
                @Function(declaration, "")
            }
        }
        @Main(topLevel)
        return Text.join(@Lines, "\n") .: "\n"
    }

    << `Def.__defaults`: sets every field to its default; construction calls it
    << first and then stores the fields that were given.
    void Defaults(Ast.Node definition) {
        @BeginFunction(true, false)
        @Alloc()
        if (definition.Value != "") {
            << Inherited fields get the base def's defaults first.
            number ignored = @Alloc()
            @Emit("CALL r" .: ignored .: " " .: @DefName(definition.Value) .: ".__defaults r0 1")
        }
        for (Ast.Node member in definition.Children) {
            if (member.Kind == Ast.NodeKind::FieldDeclaration) {
                number mark = @Next
                number value = @Alloc()
                if (len(member.Children) > 0) {
                    @Expression(member.Children[0], value)
                } else {
                    @ZeroValue(member.TypeName, value)
                }
                @Emit("SETF r0 " .: member.Name .: " r" .: value)
                @Next = mark
            }
        }
        @EndFunction(@Q(definition.Name) .: ".__defaults", 1)
    }

    void BeginFunction(bool isMethod, bool isMain) {
        @Body = array<string>({})
        @Scopes = array<Scope>()
        @Loops = array<Loop>()
        @Next = 0
        @MaxRegisters = 0
        @TryDepth = 0
        @LastLine = -1
        @InMethod = isMethod
        @InMain = isMain
        @PushScope()
    }

    void EndFunction(string name, number parameters) {
        @Emit("RETN")
        number registers = @MaxRegisters
        if (registers < parameters) {
            registers = parameters
        }
        if (registers < 1) {
            registers = 1
        }
        push(@Lines, ".func " .: name .: " " .: parameters .: " " .: registers)
        for (string line in @Body) {
            push(@Lines, line)
        }
        push(@Lines, ".end")
    }

    void Function(Ast.Node function, string owner) {
        bool isMethod = owner != ""
        @BeginFunction(isMethod, false)
        number parameters = 0
        if (isMethod) {
            @Alloc()
            parameters = 1
        }
        Ast.Node body = function.Children[len(function.Children) - 1]
        for (Ast.Node child in function.Children) {
            if (child.Kind == Ast.NodeKind::Parameter) {
                @Declare(child.Name, @Alloc())
                parameters += 1
            }
        }
        @TrackLine(function)
        for (Ast.Node statement in body.Children) {
            @Statement(statement)
        }
        string name = @Q(function.Name)
        if (isMethod) {
            name = @Q(owner) .: "." .: function.Name
        }
        @EndFunction(name, parameters)
    }

    void Main(array<Ast.Node> statements) {
        @BeginFunction(false, true)
        if (@Prefix == "") {
            << Linked modules run first, dependencies before the modules that use them.
            for (string init in @Modules.Inits) {
                number result = @Alloc()
                @Emit("CALL r" .: result .: " " .: init .: " r" .: result .: " 0")
            }
        }
        for (Ast.Node statement in statements) {
            @Statement(statement)
        }
        if (@Prefix == "") {
            @EndFunction("__main", 0)
        } else {
            @EndFunction(@Q("__init"), 0)
            push(@Modules.Inits, @Q("__init"))
        }
    }

    << ------------------------------------------------------------ statements

    void Statement(Ast.Node node) {
        @TrackLine(node)
        number mark = @Next
        Ast.NodeKind kind = node.Kind

        if (kind == Ast.NodeKind::Block) {
            @PushScope()
            for (Ast.Node child in node.Children) {
                @Statement(child)
            }
            @PopScope()
            @Next = mark
        } else if (kind == Ast.NodeKind::ExpressionStatement) {
            @Expression(node.Children[0], @Alloc())
            @Next = mark
        } else if (kind == Ast.NodeKind::VariableDeclaration) {
            @VariableDeclaration(node, mark)
        } else if (kind == Ast.NodeKind::ReturnStatement) {
            if (len(node.Children) == 0) {
                @Emit("RETN")
            } else {
                @Emit("RET r" .: @Operand(node.Children[0]))
            }
            @Next = mark
        } else if (kind == Ast.NodeKind::IfStatement) {
            string otherwise = @NewLabel()
            string done = @NewLabel()
            @Emit("JF r" .: @Operand(node.Children[0]) .: " " .: otherwise)
            @Next = mark
            @Statement(node.Children[1])
            if (len(node.Children) > 2) {
                @Emit("JMP " .: done)
                @Mark(otherwise)
                @Statement(node.Children[2])
                @Mark(done)
            } else {
                @Mark(otherwise)
            }
        } else if (kind == Ast.NodeKind::WhileStatement) {
            string start = @NewLabel()
            string done = @NewLabel()
            @Mark(start)
            @Emit("JF r" .: @Operand(node.Children[0]) .: " " .: done)
            @Next = mark
            @LoopBody(node.Children[1], start, done)
            @Emit("JMP " .: start)
            @Mark(done)
        } else if (kind == Ast.NodeKind::DoWhileStatement) {
            string start = @NewLabel()
            string again = @NewLabel()
            string done = @NewLabel()
            @Mark(start)
            @LoopBody(node.Children[0], again, done)
            @Mark(again)
            @Emit("JT r" .: @Operand(node.Children[1]) .: " " .: start)
            @Mark(done)
            @Next = mark
        } else if (kind == Ast.NodeKind::ForInStatement) {
            @ForIn(node, mark)
        } else if (kind == Ast.NodeKind::GuardStatement) {
            string ok = @NewLabel()
            @Emit("JT r" .: @Operand(node.Children[0]) .: " " .: ok)
            @Next = mark
            @Statement(node.Children[1])
            @Emit("FAIL \"guard else block must leave the scope (return, break, continue or report)\"")
            @Mark(ok)
        } else if (kind == Ast.NodeKind::TryStatement) {
            @Try(node, mark)
        } else if (kind == Ast.NodeKind::MatchStatement) {
            @Match(node, mark)
        } else if (kind == Ast.NodeKind::BreakStatement or kind == Ast.NodeKind::ContinueStatement) {
            if (len(@Loops) == 0) {
                @Fail(node, "'" .: node.Name .: "' outside of a loop")
            }
            Loop loop = @Loops[len(@Loops) - 1]
            number open = @TryDepth - loop.TryDepth
            while (open > 0) {
                @Emit("ENDTRY")
                open -= 1
            }
            if (kind == Ast.NodeKind::BreakStatement) {
                @Emit("JMP " .: loop.BreakLabel)
            } else {
                @Emit("JMP " .: loop.ContinueLabel)
            }
        } else if (
            kind == Ast.NodeKind::FunctionDeclaration
            or kind == Ast.NodeKind::DefDeclaration
            or kind == Ast.NodeKind::EnumDeclaration
        ) {
            @Fail(node, "declarations are only allowed at the top level")
        } else {
            @Fail(node, "unsupported statement " .: kind)
        }
    }

    void LoopBody(Ast.Node body, string continueLabel, string breakLabel) {
        push(@Loops, Loop { ContinueLabel = continueLabel, BreakLabel = breakLabel, TryDepth = @TryDepth })
        @Statement(body)
        pop(@Loops)
    }

    void VariableDeclaration(Ast.Node node, number mark) {
        bool global = @InMain and len(@Scopes) == 1
        number target = @Alloc()
        if (len(node.Children) > 0) {
            @Expression(node.Children[0], target)
            if (find(NUMERIC_TYPES, node.TypeName) >= 0) {
                @Convert(target, node.TypeName, @IsLiteralNumber(node.Children[0]))
            }
        } else {
            @ZeroValue(node.TypeName, target)
        }
        if (global) {
            @Emit("SETG " .: @Q(node.Name) .: " r" .: target)
            @Next = mark
        } else if (find(NUMERIC_TYPES, node.TypeName) >= 0) {
            @DeclareTyped(node.Name, target, node.TypeName)
        } else {
            @Declare(node.Name, target)
        }
    }

    void ZeroValue(string typeName, number target) {
        if (typeName == "number" or (find(NUMERIC_TYPES, typeName) >= 0 and typeName[0] != "f")) {
            @Emit("LOADI r" .: target .: " 0")
        } else if (typeName == "float" or find(NUMERIC_TYPES, typeName) >= 0) {
            @Emit("LOADF r" .: target .: " 0.0")
        } else if (typeName == "string") {
            @Emit("LOADS r" .: target .: " \"\"")
        } else if (typeName == "bool") {
            @Emit("LOADB r" .: target .: " 0")
        } else if (typeName == "array" or (len(typeName) > 6 and typeName[0 .. 6] == "array<")) {
            @Emit("NEWARR r" .: target .: " r" .: target .: " 0")
        } else if (typeName == "dictionary" or (len(typeName) > 11 and typeName[0 .. 11] == "dictionary<")) {
            @Emit("NEWDICT r" .: target .: " r" .: target .: " 0")
        } else {
            @Emit("LOADN r" .: target)
        }
    }

    void ForIn(Ast.Node node, number mark) {
        number items = @Alloc()
        @Emit("ITERLIST r" .: items .: " r" .: @Operand(node.Children[1]))
        number index = @Alloc()
        number count = @Alloc()
        number condition = @Alloc()
        number one = @Alloc()
        @Emit("LOADI r" .: index .: " 0")
        @Emit("CALLB r" .: count .: " len r" .: items .: " 1")
        @Emit("LOADI r" .: one .: " 1")
        string start = @NewLabel()
        string next = @NewLabel()
        string done = @NewLabel()
        @Mark(start)
        @Emit("LT r" .: condition .: " r" .: index .: " r" .: count)
        @Emit("JF r" .: condition .: " " .: done)
        @PushScope()
        number item = @Alloc()
        @Emit("INDEX r" .: item .: " r" .: items .: " r" .: index)
        @Declare(node.Children[0].Name, item)
        @LoopBody(node.Children[2], next, done)
        @PopScope()
        @Mark(next)
        @Emit("ADD r" .: index .: " r" .: index .: " r" .: one)
        @Emit("JMP " .: start)
        @Mark(done)
        @Next = mark
    }

    << A lambda becomes its own function whose first parameters are the
    << captured locals (by value; objects stay shared), then its own parameters.
    void Lambda(Ast.Node node, number target) {
        array<string> names = array<string>()
        array<number> registers = array<number>()
        array<string> types = array<string>()
        number depth = len(@Scopes) - 1
        while (depth >= 0) {
            Scope scope = @Scopes[depth]
            number i = len(scope.Names) - 1
            while (i >= 0) {
                if (find(names, scope.Names[i]) < 0) {
                    push(names, scope.Names[i])
                    push(registers, scope.Registers[i])
                    push(types, scope.Types[i])
                }
                i -= 1
            }
            depth -= 1
        }

        bool method = @InMethod
        array<string> savedBody = @Body
        array<Scope> savedScopes = @Scopes
        array<Loop> savedLoops = @Loops
        number savedNext = @Next
        number savedMax = @MaxRegisters
        number savedTry = @TryDepth
        number savedLine = @LastLine
        bool savedMain = @InMain

        @LambdaCount += 1
        string name = @Q("lambda" .: @LambdaCount)
        @BeginFunction(method, false)
        number parameters = 0
        if (method) {
            @Alloc()
            parameters = 1
        }
        number k = 0
        while (k < len(names)) {
            @DeclareTyped(names[k], @Alloc(), types[k])
            parameters += 1
            k += 1
        }
        for (Ast.Node child in node.Children) {
            if (child.Kind == Ast.NodeKind::Parameter) {
                @Declare(child.Name, @Alloc())
                parameters += 1
            }
        }
        @TrackLine(node)
        Ast.Node body = node.Children[len(node.Children) - 1]
        for (Ast.Node statement in body.Children) {
            @Statement(statement)
        }
        @EndFunction(name, parameters)

        @Body = savedBody
        @Scopes = savedScopes
        @Loops = savedLoops
        @Next = savedNext
        @MaxRegisters = savedMax
        @TryDepth = savedTry
        @LastLine = savedLine
        @InMain = savedMain
        @InMethod = method

        number base = @Next
        number count = len(names)
        number offset = 0
        if (method) {
            count += 1
            offset = 1
        }
        k = 0
        while (k < count) {
            @Alloc()
            k += 1
        }
        if (method) {
            @Emit("MOVE r" .: base .: " r0")
        }
        k = 0
        while (k < len(names)) {
            @Emit("MOVE r" .: (base + offset + k) .: " r" .: registers[k])
            k += 1
        }
        @Emit("CLOSURE r" .: target .: " " .: name .: " r" .: base .: " " .: count)
    }

    << The first arm with a pattern equal to the value runs; `_` always matches.
    void Match(Ast.Node node, number mark) {
        number subject = @Alloc()
        @Expression(node.Children[0], subject)
        string done = @NewLabel()
        number index = 1
        while (index < len(node.Children)) {
            Ast.Node arm = node.Children[index]
            string body = @NewLabel()
            string next = @NewLabel()
            number p = 0
            while (p < len(arm.Children) - 1) {
                Ast.Node pattern = arm.Children[p]
                if (pattern.Kind == Ast.NodeKind::IdentifierExpression and pattern.Value == "wildcard") {
                    @Emit("JMP " .: body)
                } else {
                    number patternMark = @Next
                    number value = @Operand(pattern)
                    number test = @Alloc()
                    @Emit("EQ r" .: test .: " r" .: subject .: " r" .: value)
                    @Emit("JT r" .: test .: " " .: body)
                    @Next = patternMark
                }
                p += 1
            }
            @Emit("JMP " .: next)
            @Mark(body)
            @Statement(arm.Children[len(arm.Children) - 1])
            @Emit("JMP " .: done)
            @Mark(next)
            index += 1
        }
        @Mark(done)
        @Next = mark
    }

    void Try(Ast.Node node, number mark) {
        number error = @Alloc()
        string handler = @NewLabel()
        string done = @NewLabel()
        @Emit("TRY r" .: error .: " " .: handler)
        @TryDepth += 1
        @Statement(node.Children[0])
        @TryDepth -= 1
        @Emit("ENDTRY")
        @Emit("JMP " .: done)
        @Mark(handler)
        @PushScope()
        if (len(node.Children) > 2) {
            @Declare(node.Children[2].Name, error)
        }
        @Statement(node.Children[1])
        @PopScope()
        @Mark(done)
        @Next = mark
    }

    << ------------------------------------------------------------ expressions

    << Register that holds the value: a local's own register, or a new temporary.
    number Operand(Ast.Node node) {
        if (node.Kind == Ast.NodeKind::IdentifierExpression) {
            number local = @Local(node.Name)
            if (local >= 0) {
                return local
            }
        }
        if (node.Kind == Ast.NodeKind::ThisExpression and @InMethod) {
            return 0
        }
        number target = @Alloc()
        @Expression(node, target)
        return target
    }

    string BinaryOpcode(string op) {
        if (op == "+") { return "ADD" }
        if (op == "-") { return "SUB" }
        if (op == "*") { return "MUL" }
        if (op == "/") { return "DIV" }
        if (op == "%") { return "MOD" }
        if (op == ".:") { return "CAT" }
        if (op == "==") { return "EQ" }
        if (op == "!=") { return "NE" }
        if (op == "<") { return "LT" }
        if (op == "<=") { return "LE" }
        if (op == ">") { return "GT" }
        if (op == ">=") { return "GE" }
        if (op == "&") { return "BAND" }
        if (op == "|") { return "BOR" }
        if (op == "^") { return "BXOR" }
        if (op == "shl") { return "SHL" }
        if (op == "shr") { return "SHR" }
        if (op == "..") { return "RANGE" }
        return ""
    }

    void Expression(Ast.Node node, number target) {
        Ast.NodeKind kind = node.Kind
        string r = "r" .: target

        if (kind == Ast.NodeKind::LiteralExpression) {
            @Literal(node, target)
        } else if (kind == Ast.NodeKind::IdentifierExpression) {
            number local = @Local(node.Name)
            if (local >= 0) {
                if (local != target) {
                    @Emit("MOVE " .: r .: " r" .: local)
                }
            } else if (find(@Globals, node.Name) >= 0) {
                @Emit("GETG " .: r .: " " .: @Q(node.Name))
            } else if (find(@Functions, node.Name) >= 0 or find(BUILTINS, node.Name) >= 0) {
                @Fail(node, "functions are not values yet: call " .: node.Name .: "(...)")
            } else {
                @Fail(node, "'" .: node.Name .: "' is not defined")
            }
        } else if (kind == Ast.NodeKind::ThisExpression) {
            if (not @InMethod) {
                @Fail(node, "'@this' outside of a method")
            }
            @Emit("MOVE " .: r .: " r0")
        } else if (kind == Ast.NodeKind::MemberExpression) {
            if (node.Name == "[]") {
                number container = @Operand(node.Children[0])
                number key = @Operand(node.Children[1])
                @Emit("INDEX " .: r .: " r" .: container .: " r" .: key)
            } else if (@LinkedPrefix(node.Children[0]) != "") {
                << Alias.NAME: a global of a linked module.
                @Emit("GETG " .: r .: " " .: @LinkedPrefix(node.Children[0]) .: "$" .: node.Name)
            } else if (@NativeModule(node.Children[0]) != "") {
                << Math.pi: a constant of a standard module.
                @Emit("CALLB " .: r .: " " .: @NativeModule(node.Children[0]) .: "." .: node.Name .: " " .: r .: " 0")
            } else {
                @Emit("GETF " .: r .: " r" .: @Operand(node.Children[0]) .: " " .: node.Name)
            }
        } else if (kind == Ast.NodeKind::ScopeExpression) {
            Ast.Node owner = node.Children[0]
            string enumName = ""
            if (owner.Kind == Ast.NodeKind::IdentifierExpression and find(@Enums, owner.Name) >= 0) {
                enumName = @Q(owner.Name)
            } else if (owner.Kind == Ast.NodeKind::MemberExpression and @LinkedPrefix(owner.Children[0]) != "") {
                enumName = @LinkedPrefix(owner.Children[0]) .: "$" .: owner.Name
            } else {
                @Fail(node, "'::' needs an enum class")
            }
            @Emit("ENUM " .: r .: " " .: enumName .: " " .: node.Name)
        } else if (kind == Ast.NodeKind::CallExpression) {
            @Call(node, target)
        } else if (kind == Ast.NodeKind::UnaryExpression) {
            if (node.TypeName == "postfix") {
                @Update(node.Children[0], node.Name[0 .. 1], target, true)
            } else if (node.Name == "-") {
                @Emit("NEG " .: r .: " r" .: @Operand(node.Children[0]))
            } else if (node.Name == "not" or node.Name == "!") {
                @Emit("NOT " .: r .: " r" .: @Operand(node.Children[0]))
            } else if (node.Name == "~") {
                @Emit("BNOT " .: r .: " r" .: @Operand(node.Children[0]))
            } else {
                @Expression(node.Children[0], target)
            }
        } else if (kind == Ast.NodeKind::BinaryExpression) {
            if (node.Name == "and" or node.Name == "or") {
                string done = @NewLabel()
                @Expression(node.Children[0], target)
                if (node.Name == "and") {
                    @Emit("JF " .: r .: " " .: done)
                } else {
                    @Emit("JT " .: r .: " " .: done)
                }
                @Expression(node.Children[1], target)
                @Mark(done)
            } else {
                string opcode = @BinaryOpcode(node.Name)
                if (opcode == "") {
                    @Fail(node, "unsupported operator '" .: node.Name .: "'")
                }
                number left = @Operand(node.Children[0])
                number right = @Operand(node.Children[1])
                @Emit(opcode .: " " .: r .: " r" .: left .: " r" .: right)
            }
        } else if (kind == Ast.NodeKind::Assignment) {
            if (node.Name == "=") {
                number value = @Alloc()
                @Expression(node.Children[1], value)
                @Store(node.Children[0], value)
                @Emit("MOVE " .: r .: " r" .: value)
            } else {
                @Compound(node.Children[0], node.Name[0 .. len(node.Name) - 1], node.Children[1], target)
            }
        } else if (kind == Ast.NodeKind::ConditionalExpression) {
            string otherwise = @NewLabel()
            string done = @NewLabel()
            @Emit("JF r" .: @Operand(node.Children[0]) .: " " .: otherwise)
            @Expression(node.Children[1], target)
            @Emit("JMP " .: done)
            @Mark(otherwise)
            @Expression(node.Children[2], target)
            @Mark(done)
        } else if (kind == Ast.NodeKind::ArrayLiteral) {
            number base = @Next
            for (Ast.Node item in node.Children) {
                @Alloc()
            }
            number index = 0
            while (index < len(node.Children)) {
                @Expression(node.Children[index], base + index)
                index += 1
            }
            @Emit("NEWARR " .: r .: " r" .: base .: " " .: len(node.Children))
        } else if (kind == Ast.NodeKind::DictionaryLiteral) {
            number base = @Next
            for (Ast.Node entry in node.Children) {
                @Alloc()
                @Alloc()
            }
            number index = 0
            while (index < len(node.Children)) {
                Ast.Node key = node.Children[index].Children[0]
                if (key.Kind == Ast.NodeKind::IdentifierExpression and key.Value == "bare") {
                    @Emit("LOADS r" .: (base + 2 * index) .: " \"" .: key.Name .: "\"")
                } else {
                    @Expression(key, base + 2 * index)
                }
                @Expression(node.Children[index].Children[1], base + 2 * index + 1)
                index += 1
            }
            @Emit("NEWDICT " .: r .: " r" .: base .: " " .: len(node.Children))
        } else if (kind == Ast.NodeKind::TypedCollectionExpression) {
            @Expression(node.Children[0], target)
        } else if (kind == Ast.NodeKind::TypedExpression) {
            @Expression(node.Children[0], target)
            string typeName = node.TypeName
            number dot = Text.indexOf(typeName, ".")
            if (dot > 0) {
                number alias = find(@LinkAliases, typeName[0 .. dot])
                if (alias >= 0) {
                    typeName = @LinkPrefixes[alias] .: "$" .: typeName[dot + 1 .. len(typeName)]
                }
            } else if (@FindDef(typeName).Name != "" or find(@Enums, typeName) >= 0) {
                typeName = @Q(typeName)
            }
            @Convert(target, typeName, @IsLiteralNumber(node.Children[0]))
        } else if (kind == Ast.NodeKind::ObjectConstruction) {
            @Construct(node, target)
        } else if (kind == Ast.NodeKind::LambdaExpression) {
            @Lambda(node, target)
        } else {
            @Fail(node, "unsupported expression " .: kind)
        }
    }

    void Literal(Ast.Node node, number target) {
        string r = "r" .: target
        string text = node.Value
        if (node.Name == "template") {
            @Fail(node, "template strings are not supported by the code generator yet")
        }
        if (text == "true") {
            @Emit("LOADB " .: r .: " 1")
        } else if (text == "false") {
            @Emit("LOADB " .: r .: " 0")
        } else if (text == "null") {
            @Emit("LOADN " .: r)
        } else if (text[0] == "\"" or text[0] == "'") {
            @Emit("LOADS " .: r .: " " .: text)
        } else if (
            len(text) > 1
            and (text[0 .. 2] == "0x" or text[0 .. 2] == "0X" or text[0 .. 2] == "0b" or text[0 .. 2] == "0B")
        ) {
            @Emit("LOADI " .: r .: " " .: text)
        } else if (find(text, ".") >= 0 or find(text, "e") >= 0 or find(text, "E") >= 0) {
            @Emit("LOADF " .: r .: " " .: text)
        } else {
            @Emit("LOADI " .: r .: " " .: text)
        }
    }

    void Call(Ast.Node node, number target) {
        Ast.Node callee = node.Children[0]
        number count = len(node.Children) - 1
        string head = ""

        if (callee.Kind == Ast.NodeKind::IdentifierExpression) {
            if (@Local(callee.Name) >= 0) {
                << A local holding a lambda.
                head = "CALLV r" .: target .: " r" .: @Local(callee.Name)
            } else if (find(@Functions, callee.Name) >= 0) {
                head = "CALL r" .: target .: " " .: @Q(callee.Name)
            } else if (find(BUILTINS, callee.Name) >= 0 and @Local(callee.Name) < 0) {
                head = "CALLB r" .: target .: " " .: callee.Name
            } else if (find(@Globals, callee.Name) >= 0) {
                number function = @Alloc()
                @Emit("GETG r" .: function .: " " .: @Q(callee.Name))
                head = "CALLV r" .: target .: " r" .: function
            } else if (@FindDef(callee.Name).Name != "") {
                @Fail(node, "use '" .: callee.Name .: " { Field = value }' to create a " .: callee.Name)
            } else {
                @Fail(node, "'" .: callee.Name .: "' is not a function")
            }
        } else if (callee.Kind == Ast.NodeKind::MemberExpression and @LinkedPrefix(callee.Children[0]) != "") {
            head = "CALL r" .: target .: " " .: @LinkedPrefix(callee.Children[0]) .: "$" .: callee.Name
        } else if (callee.Kind == Ast.NodeKind::MemberExpression and @NativeModule(callee.Children[0]) != "") {
            head = "CALLB r" .: target .: " " .: @NativeModule(callee.Children[0]) .: "." .: callee.Name
        } else if (callee.Kind == Ast.NodeKind::MemberExpression and callee.Name != "[]") {
            number receiver = @Operand(callee.Children[0])
            head = "CALLM r" .: target .: " r" .: receiver .: " " .: callee.Name
        } else {
            @Fail(node, "this kind of call is not supported yet")
        }

        number base = @Next
        number index = 0
        while (index < count) {
            @Alloc()
            index += 1
        }
        index = 0
        while (index < count) {
            @Expression(node.Children[index + 1], base + index)
            index += 1
        }
        @Emit(head .: " r" .: base .: " " .: count)
    }

    void Construct(Ast.Node node, number target) {
        Ast.Node typePath = node.Children[0]
        Ast.Node definition = Ast.Node { Name = "" }
        string typeName = ""
        if (typePath.Kind == Ast.NodeKind::IdentifierExpression) {
            definition = @FindDef(typePath.Name)
            typeName = @Q(typePath.Name)
        } else if (typePath.Kind == Ast.NodeKind::MemberExpression and @LinkedPrefix(typePath.Children[0]) != "") {
            string prefix = @LinkedPrefix(typePath.Children[0])
            definition = @LinkedModule(prefix).FindDef(typePath.Name)
            typeName = prefix .: "$" .: typePath.Name
        }
        if (definition.Name == "") {
            @Fail(node, "'" .: typePath.Name .: "' is not a def")
        }

        << Build into a fresh register: field values may read the target.
        number object = @Alloc()
        number ignored = @Alloc()
        @Emit("NEW r" .: object .: " " .: typeName)
        @Emit("CALLM r" .: ignored .: " r" .: object .: " __defaults r" .: ignored .: " 0")
        number index = 1
        while (index < len(node.Children)) {
            Ast.Node field = node.Children[index]
            bool known = false
            Generator owner = @this
            if (typePath.Kind == Ast.NodeKind::MemberExpression) {
                owner = @LinkedModule(@LinkedPrefix(typePath.Children[0]))
            }
            for (Ast.Node member in owner.AllFields(definition)) {
                if (member.Name == field.Name) {
                    known = true
                }
            }
            if (not known) {
                @Fail(field, "def " .: definition.Name .: " has no field '" .: field.Name .: "'")
            }
            @Emit("SETF r" .: object .: " " .: field.Name .: " r" .: @Operand(field.Children[0]))
            index += 1
        }
        @Emit("MOVE r" .: target .: " r" .: object)
    }

    << ------------------------------------------------------------ assignment

    void Store(Ast.Node place, number value) {
        if (place.Kind == Ast.NodeKind::IdentifierExpression) {
            string declared = @NumericType(place.Name)
            if (declared != "") {
                @Convert(value, declared, false)
            }
            number local = @Local(place.Name)
            if (local >= 0) {
                @Emit("MOVE r" .: local .: " r" .: value)
            } else if (find(@Globals, place.Name) >= 0) {
                @Emit("SETG " .: @Q(place.Name) .: " r" .: value)
            } else {
                @Fail(place, "'" .: place.Name .: "' is not defined")
            }
        } else if (place.Kind == Ast.NodeKind::MemberExpression and place.Name == "[]") {
            number container = @Operand(place.Children[0])
            number key = @Operand(place.Children[1])
            @Emit("SETINDEX r" .: container .: " r" .: key .: " r" .: value)
        } else if (place.Kind == Ast.NodeKind::MemberExpression) {
            @Emit("SETF r" .: @Operand(place.Children[0]) .: " " .: place.Name .: " r" .: value)
        } else {
            @Fail(place, "left side of '=' is not assignable")
        }
    }

    << `place op= value`: the container and key are evaluated once.
    void Compound(Ast.Node place, string op, Ast.Node value, number target) {
        string opcode = @BinaryOpcode(op)
        number current = @Alloc()
        if (place.Kind == Ast.NodeKind::IdentifierExpression) {
            @Expression(place, current)
            @Emit(opcode .: " r" .: current .: " r" .: current .: " r" .: @Operand(value))
            @Store(place, current)
        } else if (place.Kind == Ast.NodeKind::MemberExpression and place.Name == "[]") {
            number container = @Operand(place.Children[0])
            number key = @Operand(place.Children[1])
            @Emit("INDEX r" .: current .: " r" .: container .: " r" .: key)
            @Emit(opcode .: " r" .: current .: " r" .: current .: " r" .: @Operand(value))
            @Emit("SETINDEX r" .: container .: " r" .: key .: " r" .: current)
        } else if (place.Kind == Ast.NodeKind::MemberExpression) {
            number owner = @Operand(place.Children[0])
            @Emit("GETF r" .: current .: " r" .: owner .: " " .: place.Name)
            @Emit(opcode .: " r" .: current .: " r" .: current .: " r" .: @Operand(value))
            @Emit("SETF r" .: owner .: " " .: place.Name .: " r" .: current)
        } else {
            @Fail(place, "left side of '" .: op .: "=' is not assignable")
        }
        if (current != target) {
            @Emit("MOVE r" .: target .: " r" .: current)
        }
    }

    << `place++` / `place--`: the result is the old value.
    void Update(Ast.Node place, string sign, number target, bool postfix) {
        number one = @Alloc()
        @Emit("LOADI r" .: one .: " 1")
        number old = @Alloc()
        number fresh = @Alloc()
        string opcode = "ADD"
        if (sign == "-") {
            opcode = "SUB"
        }
        if (place.Kind == Ast.NodeKind::IdentifierExpression) {
            @Expression(place, old)
            @Emit(opcode .: " r" .: fresh .: " r" .: old .: " r" .: one)
            @Store(place, fresh)
        } else if (place.Kind == Ast.NodeKind::MemberExpression and place.Name == "[]") {
            number container = @Operand(place.Children[0])
            number key = @Operand(place.Children[1])
            @Emit("INDEX r" .: old .: " r" .: container .: " r" .: key)
            @Emit(opcode .: " r" .: fresh .: " r" .: old .: " r" .: one)
            @Emit("SETINDEX r" .: container .: " r" .: key .: " r" .: fresh)
        } else if (place.Kind == Ast.NodeKind::MemberExpression) {
            number owner = @Operand(place.Children[0])
            @Emit("GETF r" .: old .: " r" .: owner .: " " .: place.Name)
            @Emit(opcode .: " r" .: fresh .: " r" .: old .: " r" .: one)
            @Emit("SETF r" .: owner .: " " .: place.Name .: " r" .: fresh)
        } else {
            @Fail(place, "'" .: sign .: sign .: "' needs a variable, field or element")
        }
        @Emit("MOVE r" .: target .: " r" .: old)
    }
}

<<{
    !name = "Generate"
    !desc = "Compiles one parsed module to CL++ bytecode text for clpp-vm."
    !params = "module: the parsed module, tokens: its tokens (for line numbers), sourcePath: shown in runtime errors"
    !return = "the .clpb text"
}>>
export string Generate(Ast.Module module, array<TokenModule.Token> tokens, string sourcePath) {
    Generator generator = Generator {
        Prefix = "",
        Modules = Registry {
            Paths = array<string>(),
            Prefixes = array<string>(),
            Modules = array<Generator>(),
            Inits = array<string>()
        },
        LinkAliases = array<string>(),
        LinkPrefixes = array<string>(),
        SourcePath = sourcePath,
        Tokens = tokens,
        Lines = array<string>(),
        Body = array<string>(),
        Functions = array<string>(),
        Defs = array<Ast.Node>(),
        Enums = array<string>(),
        Globals = array<string>(),
        GlobalTypes = array<string>(),
        NativeAliases = array<string>(),
        NativeModules = array<string>(),
        Scopes = array<Scope>(),
        Loops = array<Loop>()
    }
    return generator.Generate(module)
}
