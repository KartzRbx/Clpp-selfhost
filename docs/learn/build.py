"""Builds the CL++ tutorial site (docs/learn/*.html) from lessons.py.

    python docs/learn/build.py           build the site
    python docs/learn/build.py --check   build into a temporary folder (CI)

Every example is run with clpp-boot and the real output is shown under it, so
the examples cannot go stale: an example that fails stops the build, unless it
is marked as an error example (then its error message is shown instead).
"""

import html
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))
from lessons import LESSONS  # noqa: E402

EXE = ROOT / "bootstrap" / "target" / "release" / ("clpp-boot.exe" if sys.platform == "win32" else "clpp-boot")

CONTROL = {
    "if", "else", "while", "do", "for", "in", "guard", "try", "catch", "break", "continue", "return",
    "match", "and", "or", "not", "shl", "shr",
}
DECLARATIONS = {"def", "enum", "class", "link", "as", "export", "const", "func", "template", "typename", "public", "private"}
TYPES = {
    "auto", "number", "float", "string", "bool", "void", "array", "dictionary", "optional",
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f16", "f32", "f64",
}
BUILTINS = {"post", "warn", "report", "len", "push", "pop", "insert", "remove", "find", "sort", "args"}
CONSTANTS = {"true", "false", "null"}

TOKEN = re.compile(
    r"(?P<comment><<\{.*?\}>>|<<\[.*?\]>>|<<[^\n]*)"
    r"|(?P<string>\"(?:[^\"\\\n]|\\.)*\"|'(?:[^'\\\n]|\\.)*'|`(?:[^`\\]|\\.)*`)"
    r"|(?P<number>\b0[xX][0-9A-Fa-f_]+\b|\b0[bB][01_]+\b|\b\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?\b)"
    r"|(?P<this>@this\b)"
    r"|(?P<member>@[A-Za-z_]\w*)"
    r"|(?P<word>[A-Za-z_]\w*)"
    r"|(?P<directive>!as\b|!style\b)"
    r"|(?P<op>\.:|::|\.\.|->|=>)"
    r"|(?P<operator>[-+*/%=<>!&|^~?:]+)"
    r"|(?P<punct>[,;.])",
    re.S,
)


def span(kind, text):
    return f'<span class="{kind}">{html.escape(text)}</span>' if kind else html.escape(text)


def word_kind(code, m):
    text = m.group(0)
    before = code[:m.start()].rstrip(" ")
    after = code[m.end():]
    calls = after.lstrip(" ").startswith("(")
    if before.endswith("::"):
        return "enum"
    if before.endswith(".") and not before.endswith(".."):
        return "call" if calls else "prop"
    if text in CONTROL:
        return "kw"
    if text in DECLARATIONS:
        return "decl"
    if text in TYPES:
        return "ty"
    if text in CONSTANTS:
        return "const"
    if text in BUILTINS and calls:
        return "fn"
    if calls:
        return "call"
    if re.fullmatch(r"[A-Z][A-Z0-9_]+", text):
        return "const"
    if text[:1].isupper():
        return "ns" if re.match(r"\.[A-Za-z_]", after) else "name"
    return "var"


def highlight(code: str) -> str:
    out = []
    last = 0
    for m in TOKEN.finditer(code):
        out.append(html.escape(code[last:m.start()]))
        text = m.group(0)
        kind = m.lastgroup
        if kind == "word":
            out.append(span(word_kind(code, m), text))
        elif kind == "member":
            calls = code[m.end():].lstrip(" ").startswith("(")
            out.append(span("this", "@") + span("call" if calls else "prop", text[1:]))
        elif kind == "string" and text.startswith("`"):
            # Template: color ${ } and keep the inside readable.
            parts = re.split(r"(\$\{[^}]*\})", text)
            for part in parts:
                if part.startswith("${"):
                    out.append(span("tpl", "${") + highlight(part[2:-1]) + span("tpl", "}"))
                else:
                    out.append(span("str", part))
        else:
            out.append(span({"comment": "comment", "string": "str", "number": "num", "this": "this", "directive": "decl", "op": "op", "operator": "oper", "punct": "punct"}[kind], text))
        last = m.end()
    out.append(html.escape(code[last:]))
    return "".join(out)


def run_example(code: str, lesson: str, expect_error: bool, files: dict) -> str:
    """Runs an example with clpp-boot and returns what it prints."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = pathlib.Path(tmp)
        for name, text in files.items():
            (tmp_path / name).write_text(text, encoding="utf-8")
        source = tmp_path / "example.clp"
        source.write_text(code, encoding="utf-8")
        env = {**dict(__import__("os").environ), "NO_COLOR": "1"}
        if expect_error:
            # Static errors (types, missing cases) come from the checker.
            checked = subprocess.run([str(EXE), "check", "example.clp"], cwd=tmp, capture_output=True, text=True, encoding="utf-8", env=env)
            if checked.returncode != 0:
                lines = [l.strip().lstrip("= ") for l in checked.stdout.splitlines() if l.startswith("error:") or "help:" in l]
                return "\n".join(lines)
        result = subprocess.run([str(EXE), "example.clp"], cwd=tmp, capture_output=True, text=True, encoding="utf-8", env=env)
        stdout = result.stdout.replace("\r\n", "\n")
        stderr = result.stderr.replace("\r\n", "\n")
        if expect_error:
            if result.returncode == 0:
                sys.exit(f"[{lesson}] example was expected to fail but succeeded:\n{code}")
            # Keep the first line of the report and its help line.
            lines = [l for l in stderr.splitlines() if l.startswith("error:") or "help:" in l]
            return stdout + "\n".join(l.strip().lstrip("= ") for l in lines)
        if result.returncode != 0:
            sys.exit(f"[{lesson}] example failed:\n{code}\n--- error ---\n{stderr}")
        # A good example must also be clean for the type checker and the editor.
        checked = subprocess.run([str(EXE), "check", "example.clp"], cwd=tmp, capture_output=True, text=True, encoding="utf-8", env=env)
        if checked.returncode != 0:
            sys.exit(f"[{lesson}] example runs but has type errors:\n{code}\n--- check ---\n{checked.stdout}")
        return stdout.rstrip("\n")


def slug(title: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", title.lower()).strip("_")


def inline(text: str) -> str:
    """`code` and **bold** in paragraphs."""
    text = html.escape(text)
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"(?<![*\w])\*([^*\s][^*]*?)\*(?![*\w])", r"<em>\1</em>", text)
    return text


def render_block(block, lesson_title):
    kind = block[0]
    if kind == "p":
        return f"<p>{inline(block[1])}</p>"
    if kind == "h2":
        return f"<h2>{inline(block[1])}</h2>"
    if kind == "note":
        return f'<div class="note"><strong>Note:</strong> {inline(block[1])}</div>'
    if kind == "tip":
        return f'<div class="tip"><strong>Tip:</strong> {inline(block[1])}</div>'
    if kind == "list":
        return "<ul>" + "".join(f"<li>{inline(item)}</li>" for item in block[1]) + "</ul>"
    if kind == "table":
        header, *rows = block[1]
        head = "".join(f"<th>{inline(h)}</th>" for h in header)
        body = "".join("<tr>" + "".join(f"<td>{inline(c)}</td>" for c in row) + "</tr>" for row in rows)
        return f'<table class="ref"><tr>{head}</tr>{body}</table>'
    if kind == "bench":
        import json
        data_path = ROOT / "benchmarks" / "results.json"
        if not data_path.exists():
            return '<div class="note"><strong>Note:</strong> run <code>python benchmarks/run.py</code> to measure this machine.</div>'
        data = json.loads(data_path.read_text(encoding="utf-8"))
        rows = "".join(
            f"<tr><td><code>{html.escape(r['name'])}.clp</code></td><td>{html.escape(r['what'])}</td>"
            f"<td>{r['interpreter']:.3f} s</td><td>{r['vm']:.3f} s</td><td><strong>{r['speedup']}×</strong></td></tr>"
            for r in data["results"]
        )
        return (
            '<table class="ref"><tr><th>Benchmark</th><th>What it measures</th><th>Interpreter</th><th>clpp-vm</th><th>Speed-up</th></tr>'
            f"{rows}</table><p>Measured on {html.escape(data['machine'])}, {data['date']}, median of {data['runs']} runs. "
            "Both runtimes printed the same output for every benchmark.</p>"
        )
    if kind == "syntax":
        return f'<div class="syntax"><pre><code>{highlight(block[1].strip())}</code></pre></div>'
    if kind in ("example", "error"):
        code = block[1].strip("\n")
        files = block[2] if len(block) > 2 else {}
        output = run_example(code, lesson_title, kind == "error", files)
        label = "Example" if kind == "example" else "Example with an error"
        out_label = "Output" if kind == "example" else "Error"
        return (
            f'<div class="example"><h3>{label}</h3>'
            f'<div class="code"><button class="copy" title="Copy">Copy</button><pre><code>{highlight(code)}</code></pre></div>'
            f'<div class="output {"bad" if kind == "error" else ""}"><span>{out_label}</span><pre>{html.escape(output) or "&nbsp;"}</pre></div></div>'
        )
    raise ValueError(f"unknown block {kind}")


CSS = """
:root{--bg:#1A1C1F;--top:#141518;--top2:#2A2C33;--side:#202227;--side-hover:#2C2E35;--accent:#CC2421;--accent2:#A51D1B;
--link:#77C821;--text:#CACACA;--strong:#ECECEC;--muted:#878787;--panel:#202227;--code:#16171A;--border:#353741;
--note:#2A2617;--note-b:#FFAA00;--tip:#1C2A1A;--tip-b:#77C821;
--kw:#5A8D7E;--decl:#CC2421;--ty:#55557F;--var:#E8C98A;--oper:#E07A5F;--punct:#A08C6E;--bracket:#C468E3;--name:#E0A458;--ns:#8FB8FF;--call:#F28B62;--fn:#FFAA00;--prop:#6EC6C3;
--this:#C468E3;--str:#77C821;--num:#D4B98C;--const:#FFC001;--com:#878787;--op:#C468E3}
:root[data-theme=light]{--bg:#FFFDF8;--top:#2A2C33;--top2:#3A3D46;--side:#F4EEE3;--side-hover:#E9E0D0;--accent:#C21F1C;--accent2:#9E1917;
--link:#3E8A0E;--text:#2B2A27;--strong:#141312;--muted:#6E6A62;--panel:#F4EEE3;--code:#FFFFFF;--border:#E2D8C6;
--note:#FFF4D6;--note-b:#E0A000;--tip:#EAF6DF;--tip-b:#5DA21A;
--var:#7A5A16;--oper:#C2491F;--punct:#7A6A50;--bracket:#8E2FB0;--kw:#2F6F5E;--decl:#B81E1B;--ty:#4B4BA8;--name:#A3621A;--ns:#2C5FB8;--call:#C2491F;--fn:#B87400;--prop:#1F7F7B;
--this:#8E2FB0;--str:#3E8A0E;--num:#8A6A2E;--const:#A87A00;--com:#7A7A7A;--op:#8E2FB0}
*{box-sizing:border-box}html,body{margin:0;background:var(--bg);color:var(--text);font:16px/1.65 "Segoe UI",Verdana,system-ui,sans-serif}
a{color:var(--link)}strong{color:var(--strong)}
header{position:sticky;top:0;z-index:10;display:flex;align-items:center;gap:18px;height:54px;padding:0 16px;background:var(--top);color:#fff;border-bottom:3px solid var(--accent)}
header .logo{display:flex;align-items:center;gap:8px;font-weight:800;font-size:22px;color:#fff;text-decoration:none;letter-spacing:.5px}
header .logo img{width:28px;height:28px;border-radius:6px}
header .logo span{color:#FFAA00}
header nav a{color:#D4D4D4;text-decoration:none;padding:16px 12px;font-size:15px}
header nav a:hover,header nav a.on{background:var(--top2);color:#fff}
header nav a.on{box-shadow:inset 0 -3px 0 #FFAA00}
header .grow{flex:1}
#menu{display:none;background:none;border:0;color:#fff;font-size:24px;cursor:pointer}
#theme{background:var(--top2);color:#fff;border:1px solid #4A4D57;border-radius:5px;padding:5px 11px;cursor:pointer}
.layout{display:flex;min-height:calc(100vh - 54px)}
aside{width:235px;flex:none;background:var(--side);padding:10px 0 40px;position:sticky;top:54px;height:calc(100vh - 54px);overflow-y:auto;border-right:1px solid var(--border)}
aside h4{margin:16px 16px 4px;font-size:13px;letter-spacing:.06em;text-transform:uppercase;color:#FFAA00}
aside a{display:block;padding:4px 16px 4px 22px;color:var(--text);text-decoration:none;font-size:15px;border-left:3px solid transparent}
aside a:hover{background:var(--side-hover);color:var(--strong)}
aside a.on{background:var(--accent);color:#fff;border-left-color:#FFAA00}
main{flex:1;min-width:0;max-width:920px;padding:18px 34px 60px}
h1{font-size:40px;font-weight:600;margin:6px 0 12px;color:var(--strong)}
h2{font-size:27px;font-weight:600;margin:34px 0 10px;color:var(--strong)}
.pager{display:flex;justify-content:space-between;margin:14px 0 18px}
.pager a{background:var(--accent);color:#fff;text-decoration:none;padding:8px 18px;border-radius:6px;font-size:15px;font-weight:600}
.pager a:hover{background:var(--accent2)}.pager span{flex:1}
hr{border:0;border-top:1px solid var(--border);margin:24px 0}
code{font-family:Consolas,"Cascadia Mono",Menlo,monospace;font-size:15px;color:#FFAA00;background:var(--panel);padding:1px 5px;border-radius:4px}
pre code{color:var(--text);background:none;padding:0}
.example{background:var(--panel);border:1px solid var(--border);border-radius:8px;padding:10px 18px 18px;margin:20px 0}
.example h3{margin:4px 0 10px;font-size:21px;font-weight:600;color:var(--strong)}
.code{position:relative;background:var(--code);border-left:4px solid var(--accent);border-radius:4px;padding:12px 16px;overflow-x:auto}
.code pre,.syntax pre,.output pre{margin:0;font-family:Consolas,"Cascadia Mono",Menlo,monospace;font-size:15px;line-height:1.55;white-space:pre}
.copy{position:absolute;top:8px;right:8px;background:var(--accent);color:#fff;border:0;border-radius:4px;padding:3px 10px;font-size:13px;cursor:pointer;opacity:.9}
.output{margin-top:10px;background:var(--code);border-left:4px solid #FFAA00;border-radius:4px;padding:8px 16px;overflow-x:auto}
.output pre{color:var(--strong)}
.output span{display:block;font-size:12px;letter-spacing:.06em;text-transform:uppercase;color:var(--muted);margin-bottom:2px}
.output.bad{border-left-color:#FF5F56}.output.bad pre{color:#FF8A84}
.syntax{background:var(--code);border-left:4px solid #77C821;border-radius:4px;padding:12px 16px;margin:14px 0;overflow-x:auto}
.note,.tip{border-left:5px solid var(--note-b);background:var(--note);padding:12px 16px;margin:16px 0;border-radius:4px}
.tip{border-left-color:var(--tip-b);background:var(--tip)}
table.ref{border-collapse:collapse;width:100%;margin:14px 0;font-size:15px}
table.ref th{background:var(--accent);color:#fff;text-align:left}
table.ref th,table.ref td{border:1px solid var(--border);padding:7px 10px;vertical-align:top}
table.ref tr:nth-child(even) td{background:var(--panel)}
.kw{color:var(--kw);font-weight:700}.decl{color:var(--decl);font-weight:700}.ty{color:var(--ty)}.name{color:var(--name)}
.ns{color:var(--ns)}.call{color:var(--call)}.fn{color:var(--fn)}.prop{color:var(--prop)}.this{color:var(--this);font-style:italic}
.str{color:var(--str)}.num{color:var(--num)}.const{color:var(--const);font-weight:700}.enum{color:var(--const);font-style:italic}
.comment{color:var(--com);font-style:italic}.var{color:var(--var)}.oper{color:var(--oper)}.punct{color:var(--punct)}
.code pre code,.syntax pre code{color:var(--bracket)}.op{color:var(--op)}.tpl{color:var(--this);font-weight:700}
.hero{background:linear-gradient(135deg,#CC2421 0%,#E0662B 55%,#FFAA00 100%);color:#fff;border-radius:10px;padding:28px 30px;margin:10px 0 22px;box-shadow:0 6px 24px #0006}
.hero h1{margin:0 0 6px;font-size:48px;color:#fff}.hero p{margin:0 0 16px;font-size:18px}
.hero a{display:inline-block;background:#1A1C1F;color:#FFAA00;font-weight:700;text-decoration:none;padding:10px 22px;border-radius:6px}
.hero a:hover{background:#000}
footer{color:var(--muted);font-size:13px;margin-top:40px}
@media (max-width:820px){#menu{display:block}aside{position:fixed;left:-260px;top:54px;z-index:9;transition:left .2s;box-shadow:2px 0 10px #0008}
aside.open{left:0}main{padding:14px 16px 50px}h1{font-size:32px}header nav{display:none}.hero h1{font-size:36px}}
"""

JS = """
for(const b of document.querySelectorAll('.copy')){b.onclick=()=>{const t=b.parentElement.querySelector('code').innerText;
navigator.clipboard.writeText(t).then(()=>{b.textContent='Copied';setTimeout(()=>b.textContent='Copy',1200)})}}
document.getElementById('menu').onclick=()=>document.querySelector('aside').classList.toggle('open');
const root=document.documentElement,saved=(()=>{try{return localStorage.getItem('clpp-theme')}catch(e){return null}})();
if(saved)root.dataset.theme=saved;
document.getElementById('theme').onclick=()=>{root.dataset.theme=root.dataset.theme==='light'?'dark':'light';
try{localStorage.setItem('clpp-theme',root.dataset.theme)}catch(e){}};
const on=document.querySelector('aside a.on');if(on)on.scrollIntoView({block:'center'});
"""


def page(lesson, index, lessons, sidebar_html):
    title = lesson["title"]
    prev_link = f'<a href="{slug(lessons[index - 1]["title"])}.html">&#10094; Previous</a>' if index > 0 else "<span></span>"
    next_link = f'<a href="{slug(lessons[index + 1]["title"])}.html">Next &#10095;</a>' if index + 1 < len(lessons) else "<span></span>"
    pager = f'<div class="pager">{prev_link}<span></span>{next_link}</div>'
    body = "\n".join(render_block(b, title) for b in lesson["blocks"])
    heading = f"<h1>{inline(lesson.get('heading', title))}</h1>" if not lesson.get("hero") else ""
    hero = ""
    if lesson.get("hero"):
        hero = (
            '<div class="hero"><h1>Learn CL++</h1><p>A small, typed language for games and tools, '
            'with a compiler written in CL++ itself.</p>'
            f'<a href="{slug(lessons[1]["title"])}.html">Start learning CL++ now &raquo;</a></div>'
        )
    return f"""<!doctype html>
<html lang="en" data-theme="dark"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>CL++ {html.escape(title)}</title><meta name="description" content="CL++ tutorial: {html.escape(title)}">
<link rel="icon" href="icon.png"><style>{CSS}</style></head>
<body><header><button id="menu" aria-label="Menu">&#9776;</button><a class="logo" href="index.html"><img src="icon.png" alt="">CL<span>++</span></a>
<nav><a class="on" href="index.html">Tutorial</a><a href="reference.html">Reference</a><a href="exercises.html">Exercises</a></nav>
<span class="grow"></span><button id="theme" title="Italian Pizza (dark) or Margherita (light)">&#127829; Theme</button></header>
<div class="layout"><aside>{sidebar_html.replace(f'href="{slug(title)}.html"', f'class="on" href="{slug(title)}.html"', 1)}</aside>
<main>{hero}{heading}{pager}<hr>
{body}
<hr>{pager}<footer>CL++ tutorial. Every example on this page was run by clpp-boot when the site was built.</footer></main></div>
<script>{JS}</script></body></html>
"""


def main():
    check = "--check" in sys.argv
    if not EXE.exists():
        sys.exit(f"build clpp-boot first: {EXE}")
    out = pathlib.Path(tempfile.mkdtemp()) if check else HERE
    sidebar = []
    group = None
    for lesson in LESSONS:
        if lesson.get("group") and lesson["group"] != group:
            group = lesson["group"]
            sidebar.append(f"<h4>{html.escape(group)}</h4>")
        sidebar.append(f'<a href="{slug(lesson["title"])}.html">{html.escape(lesson.get("menu", lesson["title"]))}</a>')
    sidebar_html = "\n".join(sidebar)
    examples = 0
    for i, lesson in enumerate(LESSONS):
        name = "index.html" if lesson.get("hero") else f"{slug(lesson['title'])}.html"
        (out / name).write_text(page(lesson, i, LESSONS, sidebar_html), encoding="utf-8")
        if lesson.get("hero"):
            (out / f"{slug(lesson['title'])}.html").write_text(page(lesson, i, LESSONS, sidebar_html), encoding="utf-8")
        examples += sum(1 for b in lesson["blocks"] if b[0] in ("example", "error"))
    icon = ROOT / "editors" / "vscode" / "images" / "icon.png"
    if icon.exists():
        shutil.copy(icon, out / "icon.png")
    print(f"{'checked' if check else 'built'} {len(LESSONS)} pages, {examples} examples run" + ("" if check else f" -> {out.relative_to(ROOT)}"))
    if check:
        shutil.rmtree(out, ignore_errors=True)


if __name__ == "__main__":
    main()
