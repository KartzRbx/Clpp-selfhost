//! Readable error reports:
//!
//! ```text
//! error: 'totl' is not defined
//!   --> game.clp:12:10
//!    |
//! 12 |     post(totl)
//!    |          ^^^^ not defined here
//!    |
//!    = help: did you mean 'total'?
//! ```

use std::io::IsTerminal;

pub struct Report {
    pub severity: &'static str,
    pub message: String,
    pub file: String,
    pub line: usize,
    /// 1-based; 0 underlines the whole line.
    pub col: usize,
    pub len: usize,
    pub help: Option<String>,
    pub trace: Vec<String>,
}

pub fn use_color() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}

struct Paint(bool);

impl Paint {
    fn wrap(&self, code: &str, text: &str) -> String {
        if self.0 {
            format!("\x1b[{}m{}\x1b[0m", code, text)
        } else {
            text.to_string()
        }
    }
}

/// Edit distance, for "did you mean" suggestions.
pub fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.to_lowercase().chars().collect();
    let b: Vec<char> = b.to_lowercase().chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut previous = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let current = row[j];
            row[j] = (row[j] + 1).min(row[j - 1] + 1).min(previous + usize::from(a[i - 1] != b[j - 1]));
            previous = current;
        }
    }
    row[b.len()]
}

/// The closest candidate, if it is close enough to be a likely typo.
pub fn closest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let limit = (name.chars().count() / 3).max(1).min(3);
    candidates
        .into_iter()
        .filter(|c| *c != name)
        .map(|c| (distance(name, c), c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c.to_string())
}

/// General advice for messages that do not carry their own.
pub fn advice(message: &str) -> Option<String> {
    let m = message;
    let tip = |s: &str| Some(s.to_string());
    if m.contains("does not join text") {
        return tip("join text with '.:', for example: \"total: \" .: score");
    }
    if m.starts_with("expected ';' or line break") {
        return tip("put each statement on its own line (';' is optional), or separate them with ';'");
    }
    if m.starts_with("condition must be bool") {
        return tip("write the comparison explicitly, for example: if (count != 0) { ... }");
    }
    if m.contains("is out of range") {
        return tip("valid indexes go from 0 to len(...) - 1; check with 'if (i < len(items))' first");
    }
    if m.starts_with("division by zero") {
        return tip("check the divisor first: guard (b != 0) else { return 0 }");
    }
    if m.contains("not found in dictionary") {
        return tip("check the key first with find(dictionary, key), or assign it before reading");
    }
    if m.contains("is not exported by") {
        return tip("add 'export' before the declaration in the linked file");
    }
    if m.starts_with("linked file not found") {
        return tip("link paths are relative to the file that contains the link, for example: link \"./utils.clp\" as Utils");
    }
    if m.contains("expected 'in' after loop variable") {
        return tip("for (number item in items) { ... } — for a counter use: for (number i in 0 .. 10) { ... }");
    }
    if m.contains("template strings are not supported") {
        return tip("build the text with '.:' instead: \"hello \" .: name");
    }
    if m.contains("functions can only be declared at the top level") {
        return tip("move the function outside of the block, or make it a method of a def");
    }
    if m.contains("guard else block must leave the scope") {
        return tip("end the else block with return, break, continue or report(...)");
    }
    if m.contains("call depth limit") {
        return tip("a function probably calls itself without a stopping condition");
    }
    if m.contains("'@' members can only be used inside a def method") || m.contains("'@this'") {
        return tip("'@Name' means '@this.Name' and only works inside methods of a def");
    }
    if m.starts_with("unterminated string") || m.starts_with("newline in quoted string") {
        return tip("close the string with a matching quote on the same line; use \\n for a line break");
    }
    if m.contains("const declaration") {
        return tip("give the constant a value: const number LIMIT = 10");
    }
    if m.contains("does not fit in") {
        return tip("use a wider type (for example i32 or i64) or check the value before converting");
    }
    None
}

pub fn render(report: &Report, source: Option<&str>) -> String {
    let p = Paint(use_color());
    let (tone, word) = match report.severity {
        "warning" => ("1;33", "warning"),
        "info" => ("1;36", "note"),
        _ => ("1;31", "error"),
    };
    let mut out = format!("{}{}\n", p.wrap(tone, &format!("{}: ", word)), p.wrap("1", &report.message));
    let line_text = source.and_then(|s| s.lines().nth(report.line.saturating_sub(1))).map(|l| l.trim_end_matches('\r').replace('\t', "    "));
    let gutter = report.line.to_string().len();
    let bar = p.wrap("1;34", "|");
    let location = if report.col > 0 {
        format!("{}:{}:{}", report.file, report.line, report.col)
    } else {
        format!("{}:{}", report.file, report.line)
    };
    out.push_str(&format!("{}{} {}\n", " ".repeat(gutter), p.wrap("1;34", "-->"), location));
    if let Some(text) = line_text {
        out.push_str(&format!("{} {}\n", " ".repeat(gutter), bar));
        out.push_str(&format!("{} {} {}\n", p.wrap("1;34", &report.line.to_string()), bar, text));
        let (start, width) = if report.col > 0 {
            (report.col - 1, report.len.max(1))
        } else {
            let indent = text.len() - text.trim_start().len();
            (indent, text.trim().chars().count().max(1))
        };
        out.push_str(&format!("{} {} {}{}\n", " ".repeat(gutter), bar, " ".repeat(start), p.wrap(tone, &"^".repeat(width))));
    }
    let help = report.help.clone().or_else(|| advice(&report.message));
    if let Some(h) = help {
        out.push_str(&format!("{} {} {}\n", " ".repeat(gutter), p.wrap("1;34", "="), p.wrap("1;36", &format!("help: {}", h))));
    }
    for frame in &report.trace {
        out.push_str(&format!("{} {} in {}\n", " ".repeat(gutter), p.wrap("1;34", "="), frame));
    }
    out
}

/// Turns "file:line:col: message" (lexer and parser errors) into a report.
pub fn from_located(text: &str) -> Option<Report> {
    // The file part may contain ':' (C:\...), so read numbers from the right.
    let marker = text.find(": ")?;
    let (head, message) = (&text[..marker], &text[marker + 2..]);
    let mut parts = head.rsplitn(3, ':');
    let col: usize = parts.next()?.parse().ok()?;
    let line: usize = parts.next()?.parse().ok()?;
    let file = parts.next()?.to_string();
    let len = message
        .rfind("(found '")
        .and_then(|i| message[i + 8..].find("')").map(|j| message[i + 8..i + 8 + j].chars().count()))
        .unwrap_or(1);
    Some(Report { severity: "error", message: message.to_string(), file, line, col, len, help: None, trace: Vec::new() })
}
