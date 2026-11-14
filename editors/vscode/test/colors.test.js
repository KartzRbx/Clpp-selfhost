'use strict';
// Colors CL++ files the way VS Code does (TextMate grammar + color theme) and
// fails when a token would be drawn in the theme's plain foreground ("white").
//
// Uses the vscode-textmate and vscode-oniguruma modules bundled with VS Code:
//   ELECTRON_RUN_AS_NODE=1 <VS Code>/Code.exe editors/vscode/test/colors.test.js <theme.json> [files...]

const fs = require('fs');
const path = require('path');

const EXT = path.join(__dirname, '..');
const ROOT = path.join(EXT, '..', '..');

function vscodeModules() {
  const appRoots = [];
  const exeDir = path.dirname(process.execPath);
  for (const entry of fs.readdirSync(exeDir, { withFileTypes: true })) {
    if (entry.isDirectory()) appRoots.push(path.join(exeDir, entry.name, 'resources', 'app'));
  }
  appRoots.push(path.join(exeDir, 'resources', 'app'));
  for (const app of appRoots) {
    const base = path.join(app, 'node_modules.asar');
    if (fs.existsSync(base)) return base;
  }
  throw new Error('run this with the Node bundled in VS Code (ELECTRON_RUN_AS_NODE=1 Code.exe ...)');
}

function stripComments(json) {
  return json.replace(/^\s*\/\/.*$/gm, '');
}

function loadTheme(themePath) {
  const theme = JSON.parse(stripComments(fs.readFileSync(themePath, 'utf8')));
  const rules = [];
  for (const rule of theme.tokenColors || []) {
    const scopes = Array.isArray(rule.scope) ? rule.scope : String(rule.scope || '').split(',');
    for (const s of scopes) {
      const selector = s.trim();
      if (selector && rule.settings && rule.settings.foreground) rules.push({ selector, color: rule.settings.foreground.toUpperCase() });
    }
  }
  return { rules, foreground: (theme.colors['editor.foreground'] || '#CACACA').toUpperCase() };
}

// TextMate selector matching, enough for themes: "a.b" matches scope "a.b.c";
// "x y" requires y on the token and x on an outer scope.
function matches(selector, stack) {
  const parts = selector.split(/\s+/);
  let i = stack.length - 1;
  const last = parts[parts.length - 1];
  const prefix = (sel, scope) => scope === sel || scope.startsWith(sel + '.');
  while (i >= 0 && !prefix(last, stack[i])) i--;
  if (i < 0) return null;
  const depth = i;
  for (let p = parts.length - 2; p >= 0; p--) {
    i--;
    while (i >= 0 && !prefix(parts[p], stack[i])) i--;
    if (i < 0) return null;
  }
  return { depth, length: last.split('.').length };
}

function colorOf(stack, theme) {
  let best = null;
  for (const rule of theme.rules) {
    const m = matches(rule.selector, stack);
    if (!m) continue;
    if (!best || m.depth > best.depth || (m.depth === best.depth && m.length >= best.length)) best = { ...m, color: rule.color };
  }
  return best ? best.color : theme.foreground;
}

async function main() {
  const args = process.argv.slice(2);
  const themePath = args[0];
  if (!themePath) throw new Error('usage: colors.test.js <theme.json> [files...]');
  const files = args.slice(1).length ? args.slice(1) : listClp(path.join(ROOT, 'src')).concat(listClp(path.join(ROOT, 'tests', 'golden')));
  const base = vscodeModules();
  const textmate = require(path.join(base, 'vscode-textmate'));
  const onig = require(path.join(base, 'vscode-oniguruma'));
  await onig.loadWASM(fs.readFileSync(path.join(base, 'vscode-oniguruma', 'release', 'onig.wasm')).buffer);
  const registry = new textmate.Registry({
    onigLib: Promise.resolve({ createOnigScanner: (p) => new onig.OnigScanner(p), createOnigString: (s) => new onig.OnigString(s) }),
    loadGrammar: async () => textmate.parseRawGrammar(fs.readFileSync(path.join(EXT, 'syntaxes', 'clpp.tmLanguage.json'), 'utf8'), 'clpp.json'),
  });
  const grammar = await registry.loadGrammar('source.clpp');
  const theme = loadTheme(themePath);
  const plain = new Map();
  let tokens = 0;
  for (const file of files) {
    let state = textmate.INITIAL;
    const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
    lines.forEach((line, n) => {
      const result = grammar.tokenizeLine(line, state);
      for (const t of result.tokens) {
        const text = line.slice(t.startIndex, t.endIndex);
        if (!text.trim()) continue;
        // Brackets are painted by VS Code's bracket-pair colorization.
        if (/^[\s(){}\[\]]+$/.test(text)) continue;
        tokens++;
        if (colorOf(t.scopes, theme) === theme.foreground) {
          const key = `${text.trim()}  [${t.scopes.slice(1).join(' ') || 'no scope'}]`;
          if (!plain.has(key)) plain.set(key, `${path.relative(ROOT, file)}:${n + 1}`);
        }
      }
      state = result.ruleStack;
    });
  }
  console.log(`${files.length} files, ${tokens} tokens, ${plain.size} kinds drawn in the plain foreground ${theme.foreground}`);
  for (const [key, where] of [...plain.entries()].slice(0, 40)) console.log(`  ${key}   e.g. ${where}`);
  process.exit(plain.size === 0 ? 0 : 1);
}

function listClp(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listClp(p));
    else if (p.endsWith('.clp')) out.push(p);
  }
  return out;
}

main().catch((e) => {
  console.error(e);
  process.exit(2);
});
