link "../../src/lexer/lexer.clp" as Lexing;
link "../../src/parser/parser.clp" as Parsing;
link "../../src/checker/naming.clp" as Naming;

array<string> CheckSource(string source) {
    return Naming.CheckNamingStyle(Parsing.Parse(source, Lexing.Lex(source)));
}

void TestNamingStyleWarnings() {
    string source =
        "!as Naming\n"
        .: "!style functions=camelCase, variables=UPPER_SNAKE_CASE, parameters=PascalCase\n"
        .: "number GetValue(number bad_name) {\n"
        .: "    number wrong_name = 1;\n"
        .: "    return 0;\n"
        .: "}\n"
        .: "number GOOD_NAME;\n";
    array<string> warnings = CheckSource(source);

    if (len(warnings) != 3) {
        report("naming style warning count mismatch");
    }

    if (
        find(warnings, "naming style warning: functions name 'GetValue' should use camelCase") < 0
        or find(warnings, "naming style warning: parameters name 'bad_name' should use PascalCase") < 0
        or find(warnings, "naming style warning: variables name 'wrong_name' should use UPPER_SNAKE_CASE") < 0
    ) {
        report("naming style warning content mismatch");
    }
}

void TestCompliantNamesHaveNoWarnings() {
    string source =
        "!as Naming;\n"
        .: "!style functions=camelCase, variables=UPPER_SNAKE_CASE, parameters=PascalCase;\n"
        .: "number getValue(number UserId) {\n"
        .: "    number MAX_COUNT = 1;\n"
        .: "    return MAX_COUNT;\n"
        .: "}\n";
    array<string> warnings = CheckSource(source);

    if (len(warnings) != 0) {
        report("compliant identifiers must not produce naming warnings");
    }
}

void TestNamingConventions() {
    string source =
        "!as Naming\n"
        .: "!style variables=UPPERCASE\n"
        .: "number camel_Name;\n";
    array<string> warnings = CheckSource(source);

    if (len(warnings) != 1) {
        report("UPPERCASE convention warning count mismatch");
    }
}

TestNamingStyleWarnings();
TestCompliantNamesHaveNoWarnings();
TestNamingConventions();
post("naming checker tests passed");
