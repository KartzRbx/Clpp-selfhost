link "../ast/ast.clp" as Ast;

const array<string> UPPER_LETTERS = array<string>(
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M",
    "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z"
);
const array<string> LOWER_LETTERS = array<string>(
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
    "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z"
);
const array<string> DIGITS = array<string>(
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9"
);

export def NamingChecker {
    bool IsUpperLetter(string character) {
        return find(UPPER_LETTERS, character) >= 0;
    }

    bool IsLowerLetter(string character) {
        return find(LOWER_LETTERS, character) >= 0;
    }

    bool IsDigit(string character) {
        return find(DIGITS, character) >= 0;
    }

    bool MatchesConvention(string name, string convention) {
        if (len(name) == 0) {
            return false;
        }

        string first = name[0 .. 1];
        if (
            (convention == "camelCase" and not @this.IsLowerLetter(first))
            or (
                (convention == "PascalCase" or convention == "UPPERCASE"
                    or convention == "UPPER_SNAKE_CASE")
                and not @this.IsUpperLetter(first)
            )
        ) {
            return false;
        }

        bool previousWasSeparator = false;
        number index = 0;
        while (index < len(name)) {
            string character = name[index];
            bool isUpper = @this.IsUpperLetter(character);
            bool isLower = @this.IsLowerLetter(character);
            bool isDigit = @this.IsDigit(character);

            if (convention == "UPPERCASE") {
                if (not isUpper and not isDigit) {
                    return false;
                }
            } else if (convention == "UPPER_SNAKE_CASE") {
                if (character == "_") {
                    if (index == 0 or index == len(name) - 1 or previousWasSeparator) {
                        return false;
                    }
                    previousWasSeparator = true;
                } else {
                    if (not isUpper and not isDigit) {
                        return false;
                    }
                    previousWasSeparator = false;
                }
            } else {
                if (not isUpper and not isLower and not isDigit) {
                    return false;
                }
            }

            index += 1;
        }

        return true;
    }

    string ConventionFor(Ast.Module module, string target) {
        number index = 0;
        while (index < len(module.NamingRules)) {
            if (module.NamingRules[index].Target == target) {
                return module.NamingRules[index].Convention;
            }
            index += 1;
        }
        return "";
    }

    string TargetFor(Ast.Node node) {
        if (node.Kind == Ast.NodeKind::FunctionDeclaration) {
            return "functions";
        }
        if (node.Kind == Ast.NodeKind::VariableDeclaration) {
            return "variables";
        }
        if (node.Kind == Ast.NodeKind::Parameter) {
            return "parameters";
        }
        return "";
    }

    array<string> CollectWarnings(
        Ast.Module module,
        Ast.Node node,
        array<string> warnings
    ) {
        string target = @this.TargetFor(node);
        string convention = @this.ConventionFor(module, target);
        if (
            target != ""
            and convention != ""
            and not @this.MatchesConvention(node.Name, convention)
        ) {
            push(
                warnings,
                "naming style warning: " .: target .: " name '"
                    .: node.Name .: "' should use " .: convention
            );
        }

        number index = 0;
        while (index < len(node.Children)) {
            warnings = @this.CollectWarnings(module, node.Children[index], warnings);
            index += 1;
        }
        return warnings;
    }
}

export array<string> CheckNamingStyle(Ast.Module module) {
    NamingChecker checker = NamingChecker {};
    array<string> warnings = array<string>();

    number index = 0;
    while (index < len(module.Declarations)) {
        warnings = checker.CollectWarnings(module, module.Declarations[index], warnings);
        index += 1;
    }

    index = 0;
    while (index < len(warnings)) {
        warn(warnings[index]);
        index += 1;
    }

    return warnings;
}
