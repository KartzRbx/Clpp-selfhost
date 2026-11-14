# CL++ Self-Hosted

Implementação experimental do compilador CL++, escrita principalmente em CL++.
O projeto está em desenvolvimento e ainda não possui uma versão executável completa.

## Estrutura

- `src/lexer/` — tokens e análise léxica.
- `src/parser/` — análise sintática e construção da AST.
- `src/ast/` — estruturas da árvore sintática.
- `src/checker/` — verificações, incluindo convenções de nomes.
- `bootstrap/` — interpretador `clpp-boot` (estágio 0) e lexer de referência em Rust.
- `tests/` — testes do lexer, parser e checker.

## Estado

O lexer, o parser inicial e a validação opcional de estilo de nomes estão em construção.
As diretivas `!style` configuram convenções para funções, variáveis e parâmetros.
Ponto e vírgula não é obrigatório: uma quebra de linha encerra declarações e
instruções; `;` continua aceito por compatibilidade.

O parser reconhece `if`/`else if`/`else`, `while`, `do ... while`,
`for (Tipo item in valores)`, `guard (cond) else { }`, `try`/`catch`, `break`,
`continue`, declarações `const`, tipos qualificados (`Ast.Node`), construção
de objetos (`Tipo { Campo = valor }`), ternário `?:`, atribuições compostas
(`+=`, `-=`...), `@this` e acesso de escopo `::`.

Dentro de métodos, `@Nome` é atalho para `@this.Nome` (`@Advance()`, `@Current += 1`).
Comentários de documentação `<<{ !desc = "..." }>>` ficam acima de uma declaração e
o parser anexa as tags ao nó (`Node.Docs`). Veja
[docs/reference/doc-comments.md](docs/reference/doc-comments.md).

## Executar e testar

`bootstrap/` contém o `clpp-boot`, um interpretador em Rust (estágio 0) que executa os
arquivos `.clp` diretamente, incluindo o próprio compilador auto-hospedado.

Na raiz do repositório (funciona no PowerShell, cmd e bash):

```bash
cargo build --release --manifest-path bootstrap/Cargo.toml
```

No Windows sem Visual Studio Build Tools, use o toolchain GNU:

```bash
cargo +stable-x86_64-pc-windows-gnu build --release --manifest-path bootstrap/Cargo.toml
```

```bash
bootstrap/target/release/clpp-boot test tests
```

```bash
bootstrap/target/release/clpp-boot programa.clp arg1 arg2
```

O `clpp-boot` não verifica tipos (isso é trabalho do checker em CL++) e ainda não
suporta `match`, lambdas, templates `` ` ``, `def` genérico nem módulos `@clpp.*`.

## Tutorial

O site de aulas fica em [docs/learn](docs/learn/index.html) (estilo w3schools: menu de lições,
exemplos com a saída real). Gere de novo com `python docs/learn/build.py`; cada exemplo é
executado pelo `clpp-boot` e verificado pelo checker durante a geração.

## Benchmarks

`python benchmarks/run.py` mede cada programa de `benchmarks/` no interpretador e no `clpp-vm`
(a saída precisa ser igual) e grava `benchmarks/results.json`, usado pela página Benchmarks do site.

## Cores no editor

`editors/vscode/test/colors.test.js` colore os arquivos `.clp` como o VS Code (gramática + tema) e
falha se algum token ficar na cor padrão do tema. Rode com o Node do VS Code:
`ELECTRON_RUN_AS_NODE=1 Code.exe editors/vscode/test/colors.test.js <tema.json>`.

## Ferramentas

| Comando | O que faz |
| --- | --- |
| `clpp-boot arquivo.clp` | Executa no interpretador (estágio 0). |
| `clpp-boot test tests` | Roda todos os testes `.clp`. |
| `clpp-boot golden` | Compila cada programa de `tests/golden` com o compilador em CL++, roda no `clpp-vm` e exige a mesma saída do interpretador. |
| `clpp-boot build entrada.clp [-o saida.clpb] [--run]` | Verifica tipos, compila (só se não houver erros) e, com `--run`, executa no `clpp-vm`. |
| `clpp-boot src/clpp.clp entrada.clp saida.clpb` | Compila para bytecode. |
| `clpp-boot bootstrap` | Auto-hospedagem: o compilador compila a si mesmo três vezes; os três bytecodes precisam ser idênticos. |
| `clpp-vm saida.clpb` | Executa o bytecode na VM de registradores. |
| `clpp-boot check arquivo.clp` | Erros sem executar: nomes, `link`, argumentos, literais fora da faixa e **tipos** (atribuições, argumentos, retornos, condições, operadores, campos), com sugestão. |
| `clpp-boot check --expect tests/typecheck/*.fail.clp` | Confere que cada `<< expect: ...` corresponde a um erro na linha. |
| `clpp-boot fmt [--check] arquivo.clp` | Formata (indentação, espaços, remove `;` sobrando). |
| `clpp-boot lsp` | Servidor de linguagem para editores. |
| `python editors/vscode/package.py` | Gera o `.vsix` com `clpp-boot`, `clpp-vm` e o compilador dentro (instale com `code --install-extension`). |

A extensão do VS Code está em [editors/vscode](editors/vscode/README.md) (ícones, cores,
erros ao digitar, hover, ir para definição, outline, autocompletar, Format Document,
`TODO`/`FIXME` no painel Problems). Neste workspace, desative a extensão oficial
`kartzrbx.clpp-language`: ela segue o CL++ 0.11 (com `;` obrigatório) e marca erros falsos.

Módulos padrão: `@clpp.math` (`pi`, `huge`, `sqrt`, `floor`, `random`...), `@clpp.text`
(`split`, `trim`, `replace`, `format`...), `@clpp.table` (`keys`, `merge`, `sorted`, `sum`...)
`@clpp.fs`, `@clpp.time` (`now`, `clock`, `sleep`, `date`, `format`) e `@clpp.json` (`parse`, `stringify`, `pretty`). Tipos numéricos: `number`, `float`, `i8`–`i64`, `u8`–`u64`, `f16`, `f32`, `f64`;
`(Tipo valor)` garante o tipo exato. Veja [docs/reference/stdlib.md](docs/reference/stdlib.md).

