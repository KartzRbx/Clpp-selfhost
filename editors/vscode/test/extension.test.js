'use strict';
// Runs extension.js against a fake `vscode` module and the real clpp-boot server.
// Build clpp-boot first, then run with any Node.js, e.g. VS Code's own:
//   ELECTRON_RUN_AS_NODE=1 code-executable editors/vscode/test/extension.test.js
const fs = require('fs');
const path = require('path');
const Module = require('module');

const EXT = path.join(__dirname, '..');
const ROOT = path.join(EXT, '..', '..').split(path.sep).join('/');
let failures = 0;
const check = (ok, label) => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}`);
  if (!ok) failures++;
};

// ---- static checks
for (const f of ['package.json', 'language-configuration.json', 'syntaxes/clpp.tmLanguage.json', 'snippets/clpp.json']) {
  try {
    JSON.parse(fs.readFileSync(path.join(EXT, f), 'utf8'));
    check(true, `valid JSON: ${f}`);
  } catch (e) {
    check(false, `valid JSON: ${f}: ${e.message}`);
  }
}
const grammar = JSON.parse(fs.readFileSync(path.join(EXT, 'syntaxes/clpp.tmLanguage.json'), 'utf8'));
let regexCount = 0;
(function walk(node) {
  if (Array.isArray(node)) return node.forEach(walk);
  if (node && typeof node === 'object') {
    for (const k of ['match', 'begin', 'end']) {
      if (typeof node[k] === 'string') {
        regexCount++;
        try {
          new RegExp(node[k]);
        } catch (e) {
          check(false, `regex ${k}: ${node[k]}: ${e.message}`);
        }
      }
    }
    Object.values(node).forEach(walk);
  }
})(grammar);
check(regexCount > 30, `${regexCount} grammar regexes compile`);
// A few tokenization spot checks with the line patterns.
const lineComment = new RegExp(grammar.repository.comments.patterns[2].match);
check(lineComment.test('<< note') && !/^<<\{/.test('<< note'), 'line comment pattern');
const decl = new RegExp(grammar.repository.declarations.patterns[3].match);
const m = decl.exec('    array<Ast.DocTag> ParseDocComment() {');
check(m && m[2] === 'array<Ast.DocTag>' && m[3] === 'ParseDocComment', 'function declaration pattern captures type and name');

// ---- fake vscode
const registered = {};
const diagnosticsByUri = new Map();
let diagResolve;
const diagArrived = new Promise((r) => (diagResolve = r));
class Position { constructor(l, c) { this.line = l; this.character = c; } }
class Range { constructor(s, e) { this.start = s; this.end = e; } }
const uriOf = (p) => ({ fsPath: p, toString: () => 'file:///' + p.replace(/\\/g, '/') });
const vscode = {
  Position, Range,
  Location: class { constructor(uri, range) { this.uri = uri; this.range = range; } },
  Hover: class { constructor(c) { this.contents = c; } },
  MarkdownString: class { constructor(v) { this.value = v; } },
  DocumentSymbol: class { constructor(name, detail, kind, range, sel) { Object.assign(this, { name, detail, kind, range, sel }); this.children = []; } },
  CompletionItem: class { constructor(label, kind) { this.label = label; this.kind = kind; } },
  TextEdit: { replace: (range, newText) => ({ range, newText }) },
  Diagnostic: class { constructor(range, message, severity) { Object.assign(this, { range, message, severity }); } },
  SymbolKind: { Module: 1, Class: 4, Method: 5, Field: 7, Enum: 9, Function: 11, Variable: 12, EnumMember: 21, Struct: 22, TypeParameter: 25 },
  CompletionItemKind: { Method: 1, Function: 2, Field: 4, Variable: 5, Class: 6, Module: 8, Enum: 12, Keyword: 13, EnumMember: 19, Struct: 21, Text: 0 },
  DiagnosticSeverity: { Error: 0, Warning: 1, Information: 2, Hint: 3 },
  Uri: { parse: (s) => ({ toString: () => s }), file: (p) => uriOf(p) },
  window: {
    createOutputChannel: () => ({ appendLine: (t) => console.log('   [output] ' + t), dispose() {} }),
    showErrorMessage: (t) => console.log('   [error] ' + t),
    showWarningMessage: () => {},
    terminals: [],
  },
  workspace: {
    workspaceFolders: [{ uri: uriOf(ROOT) }],
    textDocuments: [],
    getConfiguration: () => ({ get: () => undefined }),
    onDidOpenTextDocument: () => ({ dispose() {} }),
    onDidChangeTextDocument: () => ({ dispose() {} }),
    onDidCloseTextDocument: () => ({ dispose() {} }),
    onDidChangeConfiguration: () => ({ dispose() {} }),
  },
  languages: {
    createDiagnosticCollection: () => ({
      set: (uri, list) => { diagnosticsByUri.set(uri.toString(), list); diagResolve(); },
      delete() {}, clear() {}, dispose() {},
    }),
    registerHoverProvider: (_, p) => { registered.hover = p; return { dispose() {} }; },
    registerDefinitionProvider: (_, p) => { registered.definition = p; return { dispose() {} }; },
    registerDocumentSymbolProvider: (_, p) => { registered.symbols = p; return { dispose() {} }; },
    registerCompletionItemProvider: (_, p, ...triggers) => { registered.completion = p; registered.triggers = triggers; return { dispose() {} }; },
    registerDocumentFormattingEditProvider: (_, p) => { registered.format = p; return { dispose() {} }; },
  },
  commands: { registerCommand: (name) => { (registered.commands = registered.commands || []).push(name); return { dispose() {} }; } },
};
const originalLoad = Module._load;
Module._load = function (request, ...rest) {
  if (request === 'vscode') return vscode;
  return originalLoad.call(this, request, ...rest);
};

// A document with one deliberate error.
const file = path.join(ROOT, 'src/parser/parser.clp').replace(/\\/g, '/');
const text = fs.readFileSync(file, 'utf8');
const broken = text.replace('bool IsAtEnd() {', 'bool IsAtEnd() {\n        post(notDefinedAnywhere)');
const doc = { uri: uriOf(file), languageId: 'clpp', version: 1, getText: () => broken };
vscode.workspace.textDocuments.push(doc);

const posOf = (needle, offset) => {
  const i = broken.indexOf(needle) + offset;
  const before = broken.slice(0, i);
  return new Position(before.split('\n').length - 1, i - before.lastIndexOf('\n') - 1);
};

(async () => {
  const ext = require(path.join(EXT, 'extension.js'));
  await ext.activate({ subscriptions: [] });
  check(registered.commands.length === 3, 'commands registered: ' + registered.commands.join(', '));
  check(registered.triggers.join('') === '.@:', 'completion trigger characters');

  await Promise.race([diagArrived, new Promise((r) => setTimeout(r, 5000))]);
  const diags = diagnosticsByUri.get(doc.uri.toString()) || [];
  check(diags.length === 1 && /notDefinedAnywhere/.test(diags[0].message), `diagnostic: ${diags.map((d) => d.message).join(' | ')}`);

  const hover = await registered.hover.provideHover(doc, posOf('docs = @ParseDocComment()', 9));
  check(hover && /ParseDocComment/.test(hover.contents.value), 'hover on @ParseDocComment');

  const def = await registered.definition.provideDefinition(doc, posOf('Ast.NodeKind::DefDeclaration', 15));
  check(def && /ast\.clp$/.test(def.uri.toString()), `definition of NodeKind::DefDeclaration -> ${def && def.uri.toString()}`);

  const symbols = await registered.symbols.provideDocumentSymbols(doc);
  const parserSym = symbols.find((s) => s.name === 'Parser');
  check(parserSym && parserSym.children.length > 50, `outline: ${symbols.map((s) => s.name).join(', ')}`);

  const completions = await registered.completion.provideCompletionItems(doc, posOf('@this.Tokens[@this.Current].Kind', 6));
  check(completions.some((c) => c.label === 'Tokens'), `completion after @this. (${completions.length} items)`);

  // parser.clp ends many statements with ';', which Format Document removes.
  const edits = await registered.format.provideDocumentFormattingEdits(doc, { tabSize: 4, insertSpaces: true });
  check(
    edits.length === 1 && !/;\r?\n/.test(edits[0].newText) && edits[0].newText.includes('export def Parser {'),
    'format document returns one edit without trailing semicolons'
  );

  await ext.deactivate();
  console.log(failures === 0 ? '\nall extension checks passed' : `\n${failures} check(s) failed`);
  process.exit(failures === 0 ? 0 : 1);
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
