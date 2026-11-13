//! `clpp-boot fmt` and the language server's Format Document.
//!
//! Works on tokens and comments, so nothing is lost: comments and string
//! literals are copied verbatim. It re-indents by bracket depth (one level per
//! line, however many brackets the line opens), indents continuation lines one
//! level, normalizes spaces around operators, removes trailing `;`, and keeps
//! at most one blank line in a row. Files that do not parse are left alone.

use crate::lex::{self, Tk, Token};
use crate::parse;

pub struct Options {
    pub indent: String,
}

impl Default for Options {
    fn default() -> Self {
        Options { indent: "    ".into() }
    }
}

const BINARY: &[&str] = &[
    ".:", "+", "-", "*", "/", "%", "==", "!=", "<=", ">=", "<", ">", "&", "|", "^", "..", "?", ":", "=", "+=",
    "-=", "*=", "/=", "%=", "=>", "->", "~>",
];
const BINARY_WORDS: &[&str] = &["and", "or", "shl", "shr"];
const PAREN_KEYWORDS: &[&str] = &["if", "while", "for", "catch", "guard", "switch", "match", "return", "and", "or", "not", "in", "else"];

fn is_sym(t: &Token, s: &str) -> bool {
    t.kind == Tk::Sym && t.text == s
}

fn is_binary(t: &Token) -> bool {
    (t.kind == Tk::Sym && BINARY.contains(&t.text.as_str())) || (t.kind == Tk::Ident && BINARY_WORDS.contains(&t.text.as_str()))
}

fn is_type_token(t: &Token) -> bool {
    t.kind == Tk::Ident || (t.kind == Tk::Sym && matches!(t.text.as_str(), "." | "," | "<" | ">"))
}

/// Marks `<` and `>` that delimit type arguments, as in `array<string>`.
fn generic_marks(tokens: &[Token]) -> Vec<bool> {
    let mut marks = vec![false; tokens.len()];
    let mut i = 1;
    while i < tokens.len() {
        if is_sym(&tokens[i], "<") && tokens[i - 1].kind == Tk::Ident && !marks[i] {
            let mut depth = 0i32;
            let mut j = i;
            let mut end = None;
            while j < tokens.len() && is_type_token(&tokens[j]) {
                if is_sym(&tokens[j], "<") {
                    depth += 1;
                } else if is_sym(&tokens[j], ">") {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(j);
                        break;
                    }
                }
                j += 1;
            }
            if let Some(k) = end {
                let next = tokens.get(k + 1);
                let closes_type = match next {
                    None => true,
                    Some(n) => {
                        n.kind == Tk::Ident
                            || n.kind == Tk::Eof
                            || n.line != tokens[k].line
                            || matches!(n.text.as_str(), "(" | ")" | "," | "{" | ">" | "=" | ";" | "]")
                    }
                };
                if closes_type {
                    for (m, t) in tokens.iter().enumerate().take(k + 1).skip(i) {
                        if is_sym(t, "<") || is_sym(t, ">") {
                            marks[m] = true;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    marks
}

/// A prefix `-`, `+`, `~` or `!`.
fn is_unary(tokens: &[Token], i: usize) -> bool {
    let t = &tokens[i];
    if !(t.kind == Tk::Sym && matches!(t.text.as_str(), "-" | "+" | "~" | "!")) {
        return false;
    }
    if t.text == "!" {
        return true;
    }
    if i == 0 {
        return true;
    }
    let p = &tokens[i - 1];
    if p.line != t.line && !is_binary(p) {
        return true;
    }
    match p.kind {
        Tk::Sym => !matches!(p.text.as_str(), ")" | "]" | "}" | "++" | "--"),
        Tk::Ident => matches!(p.text.as_str(), "return" | "and" | "or" | "not" | "in" | "else"),
        _ => false,
    }
}

fn space_between(tokens: &[Token], generic: &[bool], a: usize, b: usize, line_has_question: bool) -> bool {
    let (ta, tb) = (&tokens[a], &tokens[b]);
    let sa = if ta.kind == Tk::Sym { ta.text.as_str() } else { "" };
    let sb = if tb.kind == Tk::Sym { tb.text.as_str() } else { "" };
    if matches!(sa, "(" | "[" | "." | "::" | "@") {
        return false;
    }
    if matches!(sb, ")" | "]" | "," | ";" | "." | "::") {
        return false;
    }
    if is_unary(tokens, a) {
        return false;
    }
    if generic[a] && sa == "<" {
        return false;
    }
    if generic[b] && (sb == "<" || sb == ">") {
        return false;
    }
    if generic[a] && sa == ">" {
        return !matches!(sb, "(" | ">");
    }
    if sb == "(" {
        if ta.kind == Tk::Ident {
            return PAREN_KEYWORDS.contains(&ta.text.as_str());
        }
        return !matches!(sa, ")" | "]");
    }
    if sb == "[" {
        return !(ta.kind == Tk::Ident || ta.kind == Tk::Str || matches!(sa, ")" | "]"));
    }
    if matches!(sb, "++" | "--") && (ta.kind == Tk::Ident || matches!(sa, ")" | "]")) {
        return false;
    }
    if sa == "{" && sb == "}" {
        return false;
    }
    if sb == ":" && ta.kind == Tk::Ident && !line_has_question {
        // `{ Name: string }` in a structural type.
        return false;
    }
    true
}

enum Item {
    Token(usize),
    Comment(usize),
}

pub fn format(source: &str, options: &Options) -> Result<String, String> {
    let (tokens, comments) = lex::lex_full(source, "<source>")?;
    parse::parse(tokens.clone(), "<source>", "Main")?;
    let chars: Vec<char> = source.chars().collect();
    let raw = |start: usize, end: usize| -> String { chars[start..end].iter().collect() };
    let generic = generic_marks(&tokens);
    let crlf = source.contains("\r\n");
    let source_lines: Vec<&str> = source.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let line_count = source_lines.len();

    // Lines inside a multi-line comment are copied as they are.
    let mut verbatim = vec![false; line_count + 2];
    let mut per_line: Vec<Vec<(usize, Item)>> = (0..line_count + 2).map(|_| Vec::new()).collect();
    for (i, c) in comments.iter().enumerate() {
        for l in c.line + 1..=c.end_line.min(line_count) {
            verbatim[l] = true;
        }
        per_line[c.line].push((c.start, Item::Comment(i)));
    }
    for (i, t) in tokens.iter().enumerate() {
        if t.kind != Tk::Eof && t.line <= line_count {
            per_line[t.line].push((t.start, Item::Token(i)));
        }
    }
    for items in per_line.iter_mut() {
        items.sort_by_key(|(start, _)| *start);
    }

    let mut out: Vec<String> = Vec::new();
    // Each open bracket remembers the indent level of its contents.
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut previous_last: Option<usize> = None;
    let mut pending_blank = false;

    let update_stack = |stack: &mut Vec<(char, usize)>, token_ids: &[usize], indent: usize| {
        for &i in token_ids {
            let t = &tokens[i];
            if t.kind != Tk::Sym {
                continue;
            }
            match t.text.as_str() {
                "{" | "(" | "[" => stack.push((t.text.chars().next().unwrap(), indent + 1)),
                "}" | ")" | "]" => {
                    stack.pop();
                }
                _ => {}
            }
        }
    };

    for line in 1..=line_count {
        let items = &per_line[line];
        let token_ids: Vec<usize> = items.iter().filter_map(|(_, it)| if let Item::Token(i) = it { Some(*i) } else { None }).collect();

        if verbatim[line] {
            out.push(source_lines[line - 1].trim_end().to_string());
            let indent = stack.last().map(|s| s.1).unwrap_or(0);
            update_stack(&mut stack, &token_ids, indent);
            if let Some(&last) = token_ids.last() {
                previous_last = Some(last);
            }
            continue;
        }
        if items.is_empty() {
            pending_blank = true;
            continue;
        }

        // `!as Name` / `!style ...` directives keep their own layout.
        let first_token = token_ids.first().map(|&i| &tokens[i]);
        let is_directive = matches!(items.first(), Some((_, Item::Token(_))))
            && first_token.map(|t| is_sym(t, "!")).unwrap_or(false)
            && token_ids.get(1).map(|&i| matches!(tokens[i].text.as_str(), "as" | "style")).unwrap_or(false);

        // Indentation.
        let leading_closers = token_ids
            .iter()
            .take_while(|&&i| tokens[i].kind == Tk::Sym && matches!(tokens[i].text.as_str(), "}" | ")" | "]"))
            .count();
        let mut indent = if leading_closers > 0 && !stack.is_empty() {
            let index = stack.len().saturating_sub(leading_closers.min(stack.len()));
            stack[index].1.saturating_sub(1)
        } else {
            stack.last().map(|s| s.1).unwrap_or(0)
        };
        if let (Some(first), true) = (first_token, matches!(items.first(), Some((_, Item::Token(_))))) {
            let first_index = token_ids[0];
            let starts_with_operator = is_binary(first) && !is_unary(&tokens, first_index) && !generic[first_index];
            let previous_open = previous_last
                .map(|i| tokens[i].kind == Tk::Sym && matches!(tokens[i].text.as_str(), "{" | "}" | ";"))
                .unwrap_or(true);
            let previous_continues = previous_last.map(|i| is_binary(&tokens[i]) && !generic[i]).unwrap_or(false);
            let aligned_logic = matches!(first.text.as_str(), "and" | "or") && stack.last().map(|s| s.0 == '(').unwrap_or(false);
            if leading_closers == 0 && ((starts_with_operator && !previous_open && !aligned_logic) || previous_continues) {
                indent += 1;
            }
        }

        // Drop a trailing `;` that ends a statement.
        // Brackets this line closes no longer count: `);` ends a statement.
        let open_after_closers = stack.len().saturating_sub(leading_closers);
        let inside_parens = open_after_closers > 0 && matches!(stack[open_after_closers - 1].0, '(' | '[');
        let last_token = token_ids.last().copied();
        let drop_semicolon = !inside_parens && last_token.map(|i| is_sym(&tokens[i], ";")).unwrap_or(false);

        let text = if is_directive {
            let mut t = source_lines[line - 1].trim().to_string();
            if t.ends_with(';') {
                t.pop();
            }
            t.trim_end().to_string()
        } else {
            let line_has_question = token_ids.iter().any(|&i| is_sym(&tokens[i], "?"));
            let mut text = String::new();
            let mut previous: Option<usize> = None;
            for (_, item) in items {
                match item {
                    Item::Comment(c) => {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        let c = &comments[*c];
                        // A multi-line comment contributes only its first line
                        // here; the following lines are copied verbatim.
                        let body = raw(c.start, c.end);
                        let first = body.split('\n').next().unwrap_or("").trim_end_matches('\r');
                        text.push_str(first.trim_end());
                        previous = None;
                    }
                    Item::Token(i) => {
                        if drop_semicolon && Some(*i) == last_token {
                            continue;
                        }
                        // `{ post(x); }`: the `;` before `}` is redundant.
                        if is_sym(&tokens[*i], ";") && tokens.get(*i + 1).map(|n| is_sym(n, "}") && n.line == tokens[*i].line).unwrap_or(false) {
                            continue;
                        }
                        if let Some(p) = previous {
                            if space_between(&tokens, &generic, p, *i, line_has_question) {
                                text.push(' ');
                            }
                        } else if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(&raw(tokens[*i].start, tokens[*i].end));
                        previous = Some(*i);
                    }
                }
            }
            text.trim_end().to_string()
        };

        update_stack(&mut stack, &token_ids, indent);
        if let Some(&last) = token_ids.last() {
            previous_last = Some(last);
        }
        if text.is_empty() {
            continue;
        }
        if pending_blank && !out.is_empty() {
            let after_open = out.last().map(|l| l.trim_end().ends_with('{')).unwrap_or(false);
            let before_close = leading_closers > 0 && first_token.map(|t| is_sym(t, "}")).unwrap_or(false);
            if !after_open && !before_close {
                out.push(String::new());
            }
        }
        pending_blank = false;
        out.push(format!("{}{}", options.indent.repeat(indent), text));
    }

    let newline = if crlf { "\r\n" } else { "\n" };
    let mut result = out.join(newline);
    result.push_str(newline);
    Ok(result)
}
