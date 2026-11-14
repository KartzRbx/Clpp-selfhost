'use strict';
// CL++ for VS Code: connects the editor to `clpp-boot lsp`.

const vscode = require('vscode');
const fs = require('fs');
const path = require('path');
const { LspClient } = require('./client');

let client = null;
let extensionPath = __dirname;
let output = null;
let diagnostics = null;
const versions = new Map();

function serverPath() {
  const configured = vscode.workspace.getConfiguration('clppDev').get('serverPath');
  if (configured) return configured;
  const exe = process.platform === 'win32' ? 'clpp-boot.exe' : 'clpp-boot';
  for (const folder of vscode.workspace.workspaceFolders || []) {
    const candidate = path.join(folder.uri.fsPath, 'bootstrap', 'target', 'release', exe);
    if (fs.existsSync(candidate)) return candidate;
  }
  // Packaged extension: the binaries ship in bin/<platform>-<arch>.
  const bundled = path.join(extensionPath, 'bin', `${process.platform}-${process.arch}`, exe);
  if (fs.existsSync(bundled)) return bundled;
  return 'clpp-boot';
}

function workspaceRoot() {
  const folders = vscode.workspace.workspaceFolders;
  return folders && folders.length ? folders[0].uri.fsPath : undefined;
}

// ---------------------------------------------------------------- conversions

const toPosition = (p) => new vscode.Position(p.line, p.character);
const toRange = (r) => new vscode.Range(toPosition(r.start), toPosition(r.end));
const fromPosition = (p) => ({ line: p.line, character: p.character });

const SYMBOL_KINDS = {
  2: vscode.SymbolKind.Module,
  5: vscode.SymbolKind.Class,
  6: vscode.SymbolKind.Method,
  8: vscode.SymbolKind.Field,
  10: vscode.SymbolKind.Enum,
  12: vscode.SymbolKind.Function,
  13: vscode.SymbolKind.Variable,
  22: vscode.SymbolKind.EnumMember,
  23: vscode.SymbolKind.Struct,
  26: vscode.SymbolKind.TypeParameter,
};

const COMPLETION_KINDS = {
  2: vscode.CompletionItemKind.Method,
  3: vscode.CompletionItemKind.Function,
  5: vscode.CompletionItemKind.Field,
  6: vscode.CompletionItemKind.Variable,
  7: vscode.CompletionItemKind.Class,
  9: vscode.CompletionItemKind.Module,
  13: vscode.CompletionItemKind.Enum,
  14: vscode.CompletionItemKind.Keyword,
  20: vscode.CompletionItemKind.EnumMember,
  22: vscode.CompletionItemKind.Struct,
};

function toDocumentSymbol(s) {
  const symbol = new vscode.DocumentSymbol(
    s.name,
    s.detail || '',
    SYMBOL_KINDS[s.kind] || vscode.SymbolKind.Variable,
    toRange(s.range),
    toRange(s.selectionRange)
  );
  symbol.children = (s.children || []).map(toDocumentSymbol);
  return symbol;
}

function toCompletion(item) {
  const c = new vscode.CompletionItem(item.label, COMPLETION_KINDS[item.kind] || vscode.CompletionItemKind.Text);
  if (item.detail) c.detail = item.detail;
  if (item.documentation) c.documentation = new vscode.MarkdownString(item.documentation);
  return c;
}

const SEVERITY = {
  1: vscode.DiagnosticSeverity.Error,
  2: vscode.DiagnosticSeverity.Warning,
  3: vscode.DiagnosticSeverity.Information,
  4: vscode.DiagnosticSeverity.Hint,
};

function onNotification(method, params) {
  if (method !== 'textDocument/publishDiagnostics') return;
  const uri = vscode.Uri.parse(params.uri);
  diagnostics.set(
    uri,
    params.diagnostics.map((d) => {
      const diag = new vscode.Diagnostic(toRange(d.range), d.message, SEVERITY[d.severity] || vscode.DiagnosticSeverity.Error);
      diag.source = d.source || 'clpp';
      return diag;
    })
  );
}

// ---------------------------------------------------------------- documents

const isClpp = (doc) => doc.languageId === 'clpp';

function open(doc) {
  if (!client || !isClpp(doc)) return;
  versions.set(doc.uri.toString(), doc.version);
  client.notify('textDocument/didOpen', {
    textDocument: { uri: doc.uri.toString(), languageId: 'clpp', version: doc.version, text: doc.getText() },
  });
}

function change(doc) {
  if (!client || !isClpp(doc)) return;
  client.notify('textDocument/didChange', {
    textDocument: { uri: doc.uri.toString(), version: doc.version },
    contentChanges: [{ text: doc.getText() }],
  });
}

function close(doc) {
  if (!client || !isClpp(doc)) return;
  versions.delete(doc.uri.toString());
  client.notify('textDocument/didClose', { textDocument: { uri: doc.uri.toString() } });
  diagnostics.delete(doc.uri);
}

async function request(method, doc, position, extra = {}) {
  if (!client) return null;
  try {
    return await client.request(method, {
      textDocument: { uri: doc.uri.toString() },
      ...(position ? { position: fromPosition(position) } : {}),
      ...extra,
    });
  } catch (err) {
    output.appendLine(`${method} failed: ${err.message}`);
    return null;
  }
}

// ---------------------------------------------------------------- lifecycle

async function startServer() {
  const command = serverPath();
  const trace = vscode.workspace.getConfiguration('clppDev').get('trace');
  output.appendLine(`Starting ${command} lsp`);
  client = new LspClient(command, ['lsp'], {
    cwd: workspaceRoot(),
    log: (text) => {
      if (trace) output.appendLine(text);
    },
    onNotification,
    onExit: (code) => {
      output.appendLine(`Language server stopped (${code}).`);
    },
  });
  client.start();
  try {
    await client.request('initialize', {
      processId: process.pid,
      rootUri: workspaceRoot() ? vscode.Uri.file(workspaceRoot()).toString() : null,
      capabilities: {},
    });
    client.notify('initialized', {});
    vscode.workspace.textDocuments.forEach(open);
  } catch (err) {
    vscode.window.showErrorMessage(
      `CL++: could not start the language server (${command}). Build it with "cargo build --release --manifest-path bootstrap/Cargo.toml" or set clppDev.serverPath. ${err.message}`
    );
    client = null;
  }
}

async function stopServer() {
  if (client) {
    const c = client;
    client = null;
    await c.stop();
  }
  diagnostics.clear();
}

function runInTerminal(name, args) {
  const terminal = vscode.window.terminals.find((t) => t.name === name) || vscode.window.createTerminal({ name, cwd: workspaceRoot() });
  terminal.show();
  const quoted = [serverPath(), ...args].map((a) => (/\s/.test(a) ? `"${a}"` : a));
  // PowerShell needs & to run a quoted path.
  const prefix = process.platform === 'win32' && quoted[0].startsWith('"') ? '& ' : '';
  terminal.sendText(prefix + quoted.join(' '));
}

function activate(context) {
  if (context.extensionPath) extensionPath = context.extensionPath;
  output = vscode.window.createOutputChannel('CL++');
  diagnostics = vscode.languages.createDiagnosticCollection('clpp');
  const selector = { language: 'clpp' };

  context.subscriptions.push(
    output,
    diagnostics,
    vscode.workspace.onDidOpenTextDocument(open),
    vscode.workspace.onDidChangeTextDocument((e) => change(e.document)),
    vscode.workspace.onDidCloseTextDocument(close),
    vscode.languages.registerHoverProvider(selector, {
      async provideHover(doc, position) {
        const result = await request('textDocument/hover', doc, position);
        if (!result || !result.contents) return null;
        return new vscode.Hover(new vscode.MarkdownString(result.contents.value));
      },
    }),
    vscode.languages.registerDefinitionProvider(selector, {
      async provideDefinition(doc, position) {
        const result = await request('textDocument/definition', doc, position);
        if (!result) return null;
        return new vscode.Location(vscode.Uri.parse(result.uri), toRange(result.range));
      },
    }),
    vscode.languages.registerDocumentSymbolProvider(selector, {
      async provideDocumentSymbols(doc) {
        const result = await request('textDocument/documentSymbol', doc, null);
        return (result || []).map(toDocumentSymbol);
      },
    }),
    vscode.languages.registerDocumentFormattingEditProvider(selector, {
      async provideDocumentFormattingEdits(doc, options) {
        const result = await request('textDocument/formatting', doc, null, {
          options: { tabSize: options.tabSize, insertSpaces: options.insertSpaces },
        });
        return (result || []).map((edit) => vscode.TextEdit.replace(toRange(edit.range), edit.newText));
      },
    }),
    vscode.languages.registerCompletionItemProvider(
      selector,
      {
        async provideCompletionItems(doc, position) {
          const result = await request('textDocument/completion', doc, position);
          return (result || []).map(toCompletion);
        },
      },
      '.',
      '@',
      ':'
    ),
    vscode.commands.registerCommand('clppDev.runFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || !isClpp(editor.document)) {
        vscode.window.showWarningMessage('CL++: open a .clp file first.');
        return;
      }
      await editor.document.save();
      runInTerminal('CL++', [editor.document.uri.fsPath]);
    }),
    vscode.commands.registerCommand('clppDev.runTests', () => runInTerminal('CL++ Tests', ['test', 'tests'])),
    vscode.commands.registerCommand('clppDev.restartServer', async () => {
      await stopServer();
      await startServer();
    }),
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration('clppDev.serverPath')) {
        await stopServer();
        await startServer();
      }
    })
  );

  return startServer();
}

function deactivate() {
  return stopServer();
}

module.exports = { activate, deactivate };
