# CL++ Self-Hosted

Implementação experimental do compilador CL++, escrita principalmente em CL++.
O projeto está em desenvolvimento e ainda não possui uma versão executável completa.

## Estrutura

- `src/lexer/` — tokens e análise léxica.
- `src/parser/` — análise sintática e construção da AST.
- `src/ast/` — estruturas da árvore sintática.
- `src/checker/` — verificações, incluindo convenções de nomes.
- `bootstrap/` — lexer de referência em Rust.
- `tests/` — testes do lexer, parser e checker.

## Estado

O lexer, o parser inicial e a validação opcional de estilo de nomes estão em construção.
As diretivas `!style` configuram convenções para funções, variáveis e parâmetros.
Ponto e vírgula não é obrigatório: uma quebra de linha encerra declarações e
instruções; `;` continua aceito por compatibilidade.

Não há ainda um comando de build ou execução documentado. Os testes dependem de um
runtime CL++; o bootstrap Rust possui testes próprios.
