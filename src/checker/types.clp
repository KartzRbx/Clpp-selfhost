link "../ast/ast.clp" as Ast;

export def TypeDeclarationChecker {
    bool ContainsName(array<Ast.Node> declarations, string name, number endIndex) {
        number index = 0;
        while (index < endIndex) {
            if (
                declarations[index].Kind == Ast.NodeKind::TypeDeclaration
                and declarations[index].Name == name
            ) {
                return true;
            }
            index += 1;
        }
        return false;
    }

    array<string> CheckNode(Ast.Node node, array<string> errors) {
        if (node.Kind == Ast.NodeKind::ObjectType) {
            number fieldIndex = 0;
            while (fieldIndex < len(node.Children)) {
                Ast.Node field = node.Children[fieldIndex];
                number previousIndex = 0;
                while (previousIndex < fieldIndex) {
                    if (node.Children[previousIndex].Name == field.Name) {
                        push(errors, "duplicate type field '" .: field.Name .: "'");
                        break;
                    }
                    previousIndex += 1;
                }
                fieldIndex += 1;
            }
        }

        number childIndex = 0;
        while (childIndex < len(node.Children)) {
            errors = @this.CheckNode(node.Children[childIndex], errors);
            childIndex += 1;
        }
        return errors;
    }
}

export array<string> CheckTypeDeclarations(Ast.Module module) {
    TypeDeclarationChecker checker = TypeDeclarationChecker {};
    array<string> errors = array<string>();
    number index = 0;

    while (index < len(module.Declarations)) {
        Ast.Node declaration = module.Declarations[index];
        if (declaration.Kind == Ast.NodeKind::TypeDeclaration) {
            if (checker.ContainsName(module.Declarations, declaration.Name, index)) {
                push(errors, "duplicate type declaration '" .: declaration.Name .: "'");
            }
            errors = checker.CheckNode(declaration, errors);
        }
        index += 1;
    }

    index = 0;
    while (index < len(errors)) {
        report(errors[index]);
        index += 1;
    }
    return errors;
}
