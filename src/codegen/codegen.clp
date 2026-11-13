<<[ Code generator: turns the AST of one module into CL++ bytecode (.clpb text)
    for the register virtual machine clpp-vm.

    Registers: parameters first (methods: r0 is @this), then locals, then
    temporaries. Temporaries are released at the end of every statement.
    Top-level variables become globals; top-level statements run in __main. ]>>

link "../ast/ast.clp" as Ast
link "../lexer/token.clp" as TokenModule
link @clpp.text as Text

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

export def Generator {
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
        report("codegen: " .: message .: " (line " .: line .: ")")
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
                    @Fail(declaration, "link to another file is not supported by the code generator yet")
                }
            } else if (declaration.Kind != Ast.NodeKind::TypeDeclaration) {
                push(topLevel, declaration)
            }
        }

        push(@Lines, ".clpb 1")
        if (@SourcePath != "") {
            push(@Lines, ".source " .: @SourcePath)
        }
        string moduleName = module.Name
        if (moduleName == "") {
            moduleName = "Main"
        }
        push(@Lines, ".module " .: moduleName)
        for (Ast.Node declaration in module.Declarations) {
            if (declaration.Kind == Ast.NodeKind::EnumDeclaration) {
                string line = ".enum " .: declaration.Name
                for (Ast.Node member in declaration.Children) {
                    line = line .: " " .: member.Name
                }
                push(@Lines, line)
            }
        }
        for (Ast.Node definition in @Defs) {
            string line = ".def " .: definition.Name
            for (Ast.Node member in definition.Children) {
                if (member.Kind == Ast.NodeKind::FieldDeclaration) {
                    line = line .: " " .: member.Name
                    if (find(NUMERIC_TYPES, member.TypeName) >= 0) {
                        line = line .: ":" .: member.TypeName
                    }
                }
            }
            push(@Lines, line)
        }
        for (string global in @Globals) {
            push(@Lines, ".global " .: global)
        }

        for (Ast.Node definition in @Defs) {
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
        string name = function.Name
        if (isMethod) {
            name = owner .: "." .: function.Name
        }
        @EndFunction(name, parameters)
    }

    void Main(array<Ast.Node> statements) {
        @BeginFunction(false, true)
        for (Ast.Node statement in statements) {
            @Statement(statement)
        }
        @EndFunction("__main", 0)
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
            @Emit("SETG " .: node.Name .: " r" .: target)
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
                @Emit("GETG " .: r .: " " .: node.Name)
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
            } else if (@NativeModule(node.Children[0]) != "") {
                << Math.pi: a constant of a standard module.
                @Emit("CALLB " .: r .: " " .: @NativeModule(node.Children[0]) .: "." .: node.Name .: " " .: r .: " 0")
            } else {
                @Emit("GETF " .: r .: " r" .: @Operand(node.Children[0]) .: " " .: node.Name)
            }
        } else if (kind == Ast.NodeKind::ScopeExpression) {
            Ast.Node owner = node.Children[0]
            if (owner.Kind != Ast.NodeKind::IdentifierExpression or find(@Enums, owner.Name) < 0) {
                @Fail(node, "'::' needs an enum class declared in this file")
            }
            @Emit("ENUM " .: r .: " " .: owner.Name .: " " .: node.Name)
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
            @Convert(target, node.TypeName, @IsLiteralNumber(node.Children[0]))
        } else if (kind == Ast.NodeKind::ObjectConstruction) {
            @Construct(node, target)
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
            if (find(@Functions, callee.Name) >= 0) {
                head = "CALL r" .: target .: " " .: callee.Name
            } else if (find(BUILTINS, callee.Name) >= 0 and @Local(callee.Name) < 0) {
                head = "CALLB r" .: target .: " " .: callee.Name
            } else if (@FindDef(callee.Name).Name != "") {
                @Fail(node, "use '" .: callee.Name .: " { Field = value }' to create a " .: callee.Name)
            } else {
                @Fail(node, "'" .: callee.Name .: "' is not a function")
            }
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
        if (typePath.Kind != Ast.NodeKind::IdentifierExpression) {
            @Fail(node, "only defs declared in this file can be constructed yet")
        }
        Ast.Node definition = @FindDef(typePath.Name)
        if (definition.Name == "") {
            @Fail(node, "'" .: typePath.Name .: "' is not a def declared in this file")
        }

        << Build into a fresh register: field values may read the target.
        number object = @Alloc()
        @Emit("NEW r" .: object .: " " .: definition.Name)
        array<string> provided = array<string>()
        number index = 1
        while (index < len(node.Children)) {
            Ast.Node field = node.Children[index]
            bool known = false
            for (Ast.Node member in definition.Children) {
                if (member.Kind == Ast.NodeKind::FieldDeclaration and member.Name == field.Name) {
                    known = true
                }
            }
            if (not known) {
                @Fail(field, "def " .: definition.Name .: " has no field '" .: field.Name .: "'")
            }
            @Emit("SETF r" .: object .: " " .: field.Name .: " r" .: @Operand(field.Children[0]))
            push(provided, field.Name)
            index += 1
        }
        for (Ast.Node member in definition.Children) {
            if (member.Kind == Ast.NodeKind::FieldDeclaration and find(provided, member.Name) < 0) {
                number value = @Alloc()
                if (len(member.Children) > 0) {
                    @Expression(member.Children[0], value)
                } else {
                    @ZeroValue(member.TypeName, value)
                }
                @Emit("SETF r" .: object .: " " .: member.Name .: " r" .: value)
            }
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
                @Emit("SETG " .: place.Name .: " r" .: value)
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
    !desc = "Compiles one parsed module to CL++ bytecode text for clpp-vm."
    !params = "module: the parsed module, tokens: its tokens (for line numbers), sourcePath: shown in runtime errors"
    !return = "the .clpb text"
}>>
export string Generate(Ast.Module module, array<TokenModule.Token> tokens, string sourcePath) {
    Generator generator = Generator {
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
