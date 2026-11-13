<<[ CL++ compiler driver.

    clpp-boot src/clpp.clp <input.clp> <output.clpb>
    clpp-vm <output.clpb>

    Paths are relative to the working directory.
]>>

link @clpp.fs as Fs
link "./lexer/lexer.clp" as Lexing
link "./parser/parser.clp" as Parsing
link "./codegen/codegen.clp" as Codegen

array<string> arguments = args()
guard (len(arguments) == 2) else {
    report("usage: clpp-boot src/clpp.clp <input.clp> <output.clpb>")
}

string source = Fs.read(arguments[0])
auto tokens = Lexing.Lex(source)
Fs.write(arguments[1], Codegen.Generate(Parsing.Parse(source, tokens), tokens, arguments[0]))
