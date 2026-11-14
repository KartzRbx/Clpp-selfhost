# CL++ Roadmap

This file lists everything CL++ needs to be a complete language, and how far each
part has got. Status values: **done**, **partial**, **planned**.

The compiler is written in CL++. Semicolons are optional: a line break ends a
statement, and `;` is still accepted.

## Status (2026-11-13)

Done since the first roadmap: the self-hosted parser reads all compiler sources
(`tests/selfhost`); bytecode codegen in CL++ (`src/codegen`) and the register VM
`clpp-vm` with golden tests (interpreter output == compiled output); language server
(diagnostics with suggestions, hover with doc comments, definition, outline, completion,
formatting); VS Code extension with icons; `clpp-boot fmt`; readable error reports;
`@clpp.math`, `@clpp.text`, `@clpp.table`, `@clpp.fs`; sized numbers including `f16`;
`(Type value)`; `TODO`/`FIXME` annotations.

Self-hosting reached: `clpp-boot bootstrap` compiles the compiler with itself and the
stage 1, 2 and 3 bytecode files are identical; `link` between files is compiled.

Static type checker added (gradual: unknown types never raise errors) in `clpp-boot check`
and the editor; `tests/typecheck/*.fail.clp` pin the expected errors. CI runs every suite on
Linux and Windows (`.github/workflows/ci.yml`).

`match` (with enum exhaustiveness checks) and lambdas `func (T x) -> R { }` (captures by
value) work in the interpreter, the self-hosted compiler and the VM.

Generics (`def Box<T>`, `template<typename T>`, `Box<number> { }`) and `optional<T>` with
null narrowing (`if (x != null)`, `guard (x != null)`) are checked by the type checker.

Template strings `` `Hello ${name}` `` work everywhere. `clpp-boot build` type checks before
compiling (the checker itself is still in Rust, not in CL++).

Still open, in order: type checking inside the self-hosted compiler, generics, `match`, lambdas, buffers, concurrency,
package manager, debugger.

## 0. Bootstrap (critical path)

A compiler written in CL++ needs something that already runs CL++ before it can
compile itself. `clpp-boot` is that stage 0.

| Item | Status | Notes |
| --- | --- | --- |
| Bootstrap host that runs `src/**/*.clp` | done | `clpp-boot`, a tree-walking interpreter in Rust (`bootstrap/runtime/`). No type checking. |
| Run `tests/**/*.clp` from one command | done | `clpp-boot test tests`. |
| Stage 2: the self-hosted compiler compiles itself | done | `clpp-boot bootstrap`: stages 1, 2 and 3 are byte for byte identical. |

## 1. Core language

| Item | Status | Notes |
| --- | --- | --- |
| `!as Name` module directive | done | Lexer + parser. |
| `!style` naming directives | done | Checker emits warnings. |
| `link "./file.clp" as Alias` | partial | Parsed. Module resolution and cross-file checking are not written yet. |
| Line comments `<<` and block comments `<<[ ... ]>>` | done | Lexer. |
| Doc comments `<<{ !tag = "..." }>>` | done | Lexer + parser attach tags to the next declaration. LSP hover will read them. |
| `@Name` shorthand for `@this.Name` | done | Parser + `clpp-boot`. |
| Optional `;` | done | Line break ends a statement. |
| `number`, `float`, `string`, `bool`, `void` | partial | Parsed as type names; nothing checks them yet. |
| Sized numbers `i8 i16 i32 i64 u8 u16 u32 u64 f32 f64` | planned | See section 9. |
| `array<T>`, `dictionary<K, V>`, `{ ... }` literals, empty `{}` | partial | Parser picks array vs dictionary from the declared type. Real type inference is checker work. |
| Structural types `def X = A & { F: T }`, `typeof(...)` | done | Parser + duplicate checks. |
| Nominal types `def X { fields; methods }` | planned | The compiler itself depends on this. |
| `enum class` | planned | The compiler itself depends on this. |
| `if` / `else if` / `else`, `while`, `do ... while` | done | Parser. |
| `for (T item in values)`, `break`, `continue` | done | Parser. |
| `guard (cond) else { }` | done | Parser. |
| `try` / `catch` | done | Parser. The compiler's own tests use it. |
| `match` with exhaustiveness | done | See section 10. |
| `const`, `auto`, `export` | done | Parser. |
| Functions with typed parameters and return type | done | Parser. |
| Lambdas `func (T x) -> R { }` | done | |
| Ternary `?:`, compound assignment, `@this`, `::`, `Type { Field = v }` | done | Parser. |
| Top-level statements run in order (no `main`) | planned | Runtime behaviour. |

## 2. Self-hosted compiler pipeline

| Stage | Status | Notes |
| --- | --- | --- |
| A. Lexer (index-based, line/column, escapes, comments) | done | `src/lexer/`. |
| B. Parser (recursive descent + Pratt) and AST | partial | `src/parser/`, `src/ast/`. Missing: nominal `def`, `enum class`, `match`, lambdas, generic object construction. |
| C. Type checker + symbol tables | partial | Only naming rules and duplicate type checks so far. Needs nested scopes, type compatibility, `{}` target typing, and cross-module contracts. |
| D. Codegen to register-VM bytecode | planned | Includes register allocation for locals and globals. |

## 3. Runtime and VM (`clpp-vm`)

| Item | Status |
| --- | --- |
| `clpp-vm file.clp [args...]` and `args()` | planned |
| Register-based bytecode dispatch, call frames | planned |
| REPL: `clpp-vm --repl` | planned |
| Built-ins: `post`, `warn`, `report`, `len`, `push`, ... | planned |

Performance work, in order: register VM, then computed-goto dispatch, then inline
caches for member and global lookups, then typed opcodes (`ADD_I32`, `ADD_F32`),
then contiguous `array<T>` storage. An LLVM backend for native code is a long-term
option, not a near-term goal.

## 4. Language server (`clpp-vm --lsp`)

| Item | Status |
| --- | --- |
| JSON-RPC over stdio | planned |
| `publishDiagnostics` | planned |
| Completion driven by the expected type | planned |
| Hover with types and signatures | planned |
| Index of symbols across `link`ed modules | planned |

## 5. VS Code extension

| Item | Status |
| --- | --- |
| Self-contained `.vsix` with the VM bundled (Windows x64 first) | planned |
| TextMate grammar (`!as`, `<<`, `<<[ ]>>`, keywords) | planned |
| `clpp.serverPath` setting | planned |

## 6. Tooling

`clpp build`, `clpp watch`, `clpp fmt <file>`, `clpp manifest` are all planned.
An incremental build cache keyed by a content hash of each file and its `link`
dependencies is also planned.

## 7. Tests

| Suite | Status |
| --- | --- |
| Lexer unit tests | passing under `clpp-boot` |
| Parser/AST tests | passing under `clpp-boot` |
| Checker regression tests (programs that must fail) | partial |
| Golden end-to-end tests (compile, run, compare output) | planned |

## 8. Documentation

English reference pages live in [`docs/`](docs/). Every page follows the same
layout: syntax in one line, a complete runnable example, the expected output, then
common mistakes. Aim for about 80% examples and 20% explanation.

## 9. Sized numeric types (proposal)

| Type | Range |
| --- | --- |
| `i8` / `i16` / `i32` / `i64` | -2^(n-1) .. 2^(n-1)-1 |
| `u8` / `u16` / `u32` / `u64` | 0 .. 2^n-1 |
| `f32` / `f64` | IEEE 754 single / double |

Rules:

- `number` stays as the name for `i64`, and `float` as the name for `f64`.
- If a literal does not fit its declared type, the compiler warns. At run time,
  overflow wraps (two's complement) unless strict debug checks are on.
- In mixed integer arithmetic, the narrower operand widens to the wider type.
- Assigning a negative literal to a `u*` type is a compile error.
- Integers and floats never convert implicitly: use `static_cast<f32>(x)` or a
  float literal (`10.0`).

## 10. Advanced features (proposals)

| Feature | Sketch |
| --- | --- |
| `match` | `match (status) { "active" => post("ok"), _ => post("?") }`, checked for exhaustiveness on enums and literal unions |
| Flow-sensitive narrowing | After `if (input != null)`, `optional<string>` narrows to `string` |
| Errors as values | `Result<T, E>` plus a postfix `?` that propagates the error; `try`/`catch` stays |
| `comptime` | `number size = comptime { return 1024 * 512 }` is evaluated by the compiler |
| FFI | `extern "C" { u32 get_system_ticks() }` |
| Arena allocation | Free a whole block of memory at once, at the end of a frame or task |
| Hygienic macros | `macro Serialize(Target) { ... }` |

## Design decisions

Some proposals disagreed with syntax the language already uses. The existing
syntax was kept in each case:

| Proposal | Kept | Reason |
| --- | --- | --- |
| `boolean` | `bool` | The lexer and all existing code use `bool`. |
| `struct` | `def` | `def` is the only data keyword. |
| `//` comments | `<<` | `<<` is the CL++ comment syntax. |
| `<<` / `>>` bit shifts | `shl` / `shr` | `<<` already starts a comment. |
| `string \| void` and `!= void` | `optional<string>` and `null` | `void` is a return type only, not a value. |
| `10.0f` suffix | `10.0` with `static_cast<f32>` | No literal suffixes until the sized types are designed. |

## 11. Gap review against Rust, TypeScript, Python, Go and C++

Reviewed 2026-11-13. ✅ = has it, 🟡 = partial or only specified, ❌ = missing.
The CL++ column describes this repository, not the official C++20 checkout.

### Language

| Feature | Rust | TS | Python | Go | C++ | CL++ | What CL++ needs |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Memory model | ownership | GC | GC + refcount | GC | RAII / manual | ❌ | Decide: tracing GC, refcount + cycle collector, or arenas. `clpp-boot` uses refcounting and leaks cycles. |
| Static types | ✅ | ✅ | hints only | ✅ | ✅ | 🟡 | Real type checker (stage C). |
| Generics | ✅ | ✅ | ✅ | ✅ | templates | ✅ | `def Box<T>`, generic functions, bounds. |
| Interfaces / traits | traits | interfaces | protocols | interfaces | concepts | ❌ | A contract mechanism: structural like Go/TS fits the existing `&` types. |
| Enums with data | ✅ | unions | ❌ | ❌ | `variant` | 🟡 | Payload unions `def Shape = Circle \| Square` + `match`. |
| Pattern matching | ✅ | narrowing | `match` | `switch` | ❌ | ✅ | `match` with exhaustiveness. |
| Null safety | `Option` | strict null | ❌ | ❌ | `optional` | ✅ | `optional<T>` + flow narrowing. |
| Error handling | `Result` + `?` | exceptions | exceptions | error values | exceptions | 🟡 | `try`/`catch` exists; add `Result<T, E>` + `?`. Errors carry only a string today. |
| Closures | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | `func (T x) -> R { }` capturing variables. |
| Iterators / generators | ✅ | ✅ | ✅ | range-over-func | ranges | 🟡 | `for-in` only over arrays/dicts/strings; add an iterator protocol. |
| String API | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | split, trim, contains, replace, format, upper/lower, char codes. |
| String interpolation | `format!` | template | f-strings | `Sprintf` | `std::format` | ✅ | Lexer has `` `${}` ``; parser and `clpp-boot` don't evaluate it. |
| Sized numbers | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ | `i8`…`f64` (section 9). |
| Concurrency | async + threads | async | asyncio | goroutines | threads | ❌ | Pick one model: tasks + channels (Go) suits game logic. |
| Operator overloading | traits | ❌ | dunders | ❌ | ✅ | ❌ | Decide yes/no (`operator` is already a keyword). |
| Attributes / decorators | `#[...]` | ✅ | ✅ | tags | `[[...]]` | ❌ | Needed for `test`, `deprecated`, `inline`, serialization. |
| Macros / comptime | ✅ | ❌ | ❌ | `go generate` | templates/constexpr | ❌ | Section 10. |
| Visibility | ✅ | ✅ | convention | case | ✅ | 🟡 | `export` enforced by `clpp-boot`; `public`/`private` parsed but not enforced. |
| Destructors / cleanup | `Drop` | ❌ | `with` | `defer` | RAII | ❌ | `defer` is the simplest fit. |
| FFI | ✅ | N-API | C API | cgo | native | ❌ | `extern "C"` (section 10). |

### Tooling

| Tool | Rust | TS | Python | Go | C++ | CL++ | What CL++ needs |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Compiler driver | `rustc` | `tsc` | `python` | `go` | `clang` | 🟡 | `clpp-boot` runs files; the self-hosted `clpp` driver does not exist. |
| Package manager | cargo | npm | pip / uv | go mod | vcpkg / conan | ❌ | Manifest (`clpp.toml`), lockfile, dependency resolution. |
| Package registry | crates.io | npm | PyPI | proxy | ❌ | ❌ | Can start as git URLs. |
| Build system | cargo | tsc / bundlers | ❌ | go build | CMake | ❌ | `clpp build` over a project. |
| Formatter | rustfmt | prettier | black / ruff | gofmt | clang-format | ❌ | `clpp fmt`, one canonical style (gofmt approach). |
| Linter | clippy | eslint | ruff | go vet | clang-tidy | 🟡 | Naming checker only. |
| Test framework | `cargo test` | jest / vitest | pytest | `go test` | gtest | 🟡 | `clpp-boot test` runs files. Needs `assert`, per-function test discovery, failure diffs. |
| Benchmarks | criterion | ✅ | ✅ | `go test -bench` | ✅ | ❌ | |
| Doc generator | rustdoc | TypeDoc | Sphinx | godoc | Doxygen | 🟡 | `<<{ }>>` doc comments are parsed; no generator yet. |
| REPL | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | `clpp-vm --repl`. |
| Debugger | gdb / lldb | Chrome DevTools | pdb | delve | gdb / lldb | ❌ | Line tables in bytecode + Debug Adapter Protocol. |
| Profiler | perf | DevTools | cProfile | pprof | perf | ❌ | Sampling profiler in the VM. |
| Diagnostics quality | excellent | good | good | good | poor | 🟡 | Error codes, source line with `^` caret, suggestions. |
| Incremental builds | ✅ | ✅ | `.pyc` | ✅ | ✅ | ❌ | Section 6. |
| Online playground | ✅ | ✅ | ✅ | ✅ | Compiler Explorer | ❌ | Compile the VM to WebAssembly. |

### IDE

| Feature | Rust | TS | Python | Go | C++ | CL++ |
| --- | --- | --- | --- | --- | --- | --- |
| Syntax highlighting (TextMate) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Tree-sitter grammar (Neovim, Zed, GitHub) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Language server | rust-analyzer | tsserver | Pylance | gopls | clangd | ❌ |
| Diagnostics while typing | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Completion | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Hover (types + doc comments) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Go to definition / find references | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Rename symbol | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Signature help | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Inlay hints (inferred `auto` types) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Code actions / quick fixes | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Format on save | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Debugging in the editor (DAP) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Run / test from the editor | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Snippets | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |

The language server must reuse the compiler's lexer, parser and checker, as
rust-analyzer and tsserver do. That needs an error-tolerant parser (it keeps going
after the first error) and positions on every AST node. Both are missing today:
`report` stops at the first error.

### Standard library

| Area | CL++ |
| --- | --- |
| Collections: array, dictionary | 🟡 basic built-ins only |
| Collections: set, queue, stack, sorted map | ❌ |
| Strings, char codes, UTF-8 | ❌ |
| Math (`Sqrt`, `Floor`, `Random`, ...) | ❌ |
| Files and paths (`@clpp.fs`, `@clpp.path`) | ❌ (also needed by the compiler itself) |
| Process, args, environment, exit codes | 🟡 `args()` only |
| Time and dates | ❌ |
| JSON | ❌ |
| Regular expressions | ❌ |
| Networking / HTTP | ❌ |

### Project infrastructure

| Item | CL++ |
| --- | --- |
| CI that runs `clpp-boot test` on every push | ❌ |
| LICENSE | ❌ |
| CONTRIBUTING guide | ❌ |
| Formal grammar (EBNF) in the repo | ❌ |
| Language specification | ❌ (site documents the C++20 checkout) |
| Versioning and stability policy (Rust-style editions) | ❌ |
| Changelog | ❌ |
| Examples folder (`examples/*.clp`) used as golden tests | ❌ |

### Suggested order

1. **Self-hosting:** the compiler must parse its own sources (`enum class`, nominal
   `def` bodies, multi-argument `array<T>(...)`, the `\r` escape bug).
2. **Type checker:** generics, interfaces and `optional` all depend on it.
3. **Tests, CI and diagnostics:** an `assert` built-in, CI, and error messages
   with a caret under the problem.
4. **Language server + VS Code extension:** highlighting first, then diagnostics,
   hover, completion and go-to-definition.
5. **Standard library basics:** strings, math, files.
6. **Bytecode codegen + `clpp-vm`**, then the debugger (DAP) and REPL.
7. **Package manager**, formatter, doc generator.
8. Concurrency, FFI, macros/comptime, LLVM.
