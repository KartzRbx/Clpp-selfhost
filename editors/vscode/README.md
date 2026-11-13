# CL++ for VS Code

Language support for `.clp` files.

| Feature | How |
| --- | --- |
| Syntax highlighting | TextMate grammar: keywords, types, `<<` / `<<[ ]>>` comments, `<<{ !tag }>>` doc comments, `!as`, `@Name`, `Enum::Member`, `.:` |
| Errors while you type | syntax errors, undefined names, missing `link` files, members that are not exported, unknown `@Name` / `Type { Field }` / `Enum::X`, wrong argument counts |
| Hover | signature and `<<{ }>>` doc comment of functions, methods, fields, defs, enums and built-ins |
| Go to definition | F12 on any name, also across `link`ed files; on a `link "path"` it opens the file |
| Outline | defs with their fields and methods, functions, enums, globals |
| Completion | after `@`, `@this.`, `Alias.`, `Enum::`, `variable.` (uses the variable's declared type), and everywhere else |
| Snippets | `def`, `fn`, `efn`, `if`, `ife`, `while`, `loop`, `for`, `do`, `guard`, `try`, `enum`, `link`, `doc`, `test`, `post` |
| Commands | **CL++: Run File** (Ctrl+F5), **CL++: Run Tests**, **CL++: Restart Language Server** |

The extension has no npm dependencies. It starts `clpp-boot lsp`, a language server
over stdio, so the same server also works in Neovim, Helix, Zed or any editor that
speaks LSP.

## Install

Build the server from the repository root:

```bash
cargo build --release --manifest-path bootstrap/Cargo.toml
```

Copy this folder into your VS Code extensions folder (PowerShell, from the repository root):

```bash
Copy-Item -Recurse -Force editors\vscode "$env:USERPROFILE\.vscode\extensions\clpp-local.clpp-0.1.0"
```

Then run **Developer: Reload Window**.

The extension looks for the server in this order:

1. the `clpp.serverPath` setting;
2. `bootstrap/target/release/clpp-boot(.exe)` in the open workspace;
3. `clpp-boot` on `PATH`.

## Test

```bash
ELECTRON_RUN_AS_NODE=1 code-path/Code.exe editors/vscode/test/extension.test.js
```

The test loads `extension.js` with a fake `vscode` module, starts the real server and
checks diagnostics, hover, go to definition, outline and completion on
`src/parser/parser.clp`.

## Other editors

Point the editor's LSP client at `clpp-boot lsp` for files ending in `.clp`. Neovim:

```lua
vim.filetype.add({ extension = { clp = "clpp" } })
vim.lsp.start({ name = "clpp", cmd = { "clpp-boot", "lsp" }, filetypes = { "clpp" } })
```
