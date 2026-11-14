//! `clpp-boot`: stage-0 runner for the self-hosted CL++ compiler.
//!
//! Usage:
//!   clpp-boot <file.clp> [args...]   run a program
//!   clpp-boot test [dir]             run every .clp file under dir (default: tests)
//!   clpp-boot check <file.clp>...    report errors without running
//!   clpp-boot fmt [--check] <file>...  format files in place
//!   clpp-boot lsp                    language server over stdio
//!   clpp-boot bootstrap              the compiler compiles itself; stages must match
//!   clpp-boot golden [dir]           interpreter vs compiled bytecode (default: tests/golden)

mod analysis;
mod ast;
mod diag;
mod fmt;
mod interp;
mod json;
mod lex;
mod lsp;
mod parse;
mod stdlib;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const STACK_SIZE: usize = 512 * 1024 * 1024;

fn run(path: &Path, args: Vec<String>) -> Result<(), String> {
    let mut interp = interp::Interp::new(args);
    interp.run_file(path)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(e) => e.filter_map(|e| e.ok().map(|e| e.path())).collect(),
        Err(_) => return,
    };
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().map(|e| e == "clp").unwrap_or(false) {
            out.push(path);
        }
    }
}

fn run_tests(dir: &Path) -> bool {
    let mut files = Vec::new();
    collect(dir, &mut files);
    if files.is_empty() {
        eprintln!("no .clp files found under {}", dir.display());
        return false;
    }
    let mut failed = 0;
    for file in &files {
        println!("== {}", file.display());
        match run(file, Vec::new()) {
            Ok(()) => println!("   PASS"),
            Err(message) => {
                failed += 1;
                println!("   FAIL");
                for line in message.lines() {
                    println!("   {}", line);
                }
            }
        }
    }
    println!("\n{} passed, {} failed, {} total", files.len() - failed, failed, files.len());
    failed == 0
}

/// Differential test: every program must print the same in the interpreter
/// and when compiled by the self-hosted compiler and run on clpp-vm. A
/// `<name>.out` file next to a program pins the expected output.
fn golden(dir: &Path) -> bool {
    use std::process::Command;
    let exe = std::env::current_exe().expect("current exe");
    let vm = exe.with_file_name(if cfg!(windows) { "clpp-vm.exe" } else { "clpp-vm" });
    if !vm.exists() {
        eprintln!("clpp-vm not found next to clpp-boot: build both with cargo build --release");
        return false;
    }
    let mut files = Vec::new();
    collect(dir, &mut files);
    let tmp = std::env::temp_dir().join(format!("clpp-golden-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    let mut failed = 0;
    let run = |cmd: &mut Command| match cmd.output() {
        Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stdout).replace("\r\n", "\n"), String::from_utf8_lossy(&o.stderr).to_string()),
        Err(e) => (false, String::new(), e.to_string()),
    };
    for file in &files {
        println!("== {}", file.display());
        let bytecode = tmp.join(file.file_stem().unwrap()).with_extension("clpb");
        let (interp_ok, interp_out, interp_err) = run(Command::new(&exe).arg(file));
        let (compiled, _, compile_err) = run(Command::new(&exe).arg("src/clpp.clp").arg(file).arg(&bytecode));
        let mut problems = Vec::new();
        if !compiled {
            problems.push(format!("compile failed: {}", compile_err.trim()));
        } else {
            let (vm_ok, vm_out, vm_err) = run(Command::new(&vm).arg(&bytecode));
            if vm_ok != interp_ok {
                problems.push(format!(
                    "exit status differs: interpreter {}, vm {}\n{}{}",
                    if interp_ok { "ok" } else { "failed" },
                    if vm_ok { "ok" } else { "failed" },
                    interp_err.trim(),
                    vm_err.trim()
                ));
            }
            if vm_out != interp_out {
                let line = vm_out.lines().zip(interp_out.lines()).position(|(a, b)| a != b).unwrap_or(vm_out.lines().count().min(interp_out.lines().count()));
                problems.push(format!(
                    "output differs at line {}:\n  interpreter: {}\n  vm:          {}",
                    line + 1,
                    interp_out.lines().nth(line).unwrap_or("<end>"),
                    vm_out.lines().nth(line).unwrap_or("<end>")
                ));
            }
        }
        let expected_path = file.with_extension("out");
        if let Ok(expected) = std::fs::read_to_string(&expected_path) {
            if expected.replace("\r\n", "\n") != interp_out {
                problems.push(format!("output differs from {}", expected_path.display()));
            }
        }
        if problems.is_empty() {
            println!("   PASS");
        } else {
            failed += 1;
            println!("   FAIL");
            for p in problems {
                for line in p.lines() {
                    println!("   {}", line);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    println!("\n{} passed, {} failed, {} total", files.len() - failed, failed, files.len());
    failed == 0 && !files.is_empty()
}

/// Self-hosting check: the compiler compiled by itself must reproduce itself.
///   stage 1: clpp-boot runs src/clpp.clp to compile src/clpp.clp
///   stage 2: clpp-vm runs stage 1 to compile src/clpp.clp
///   stage 3: clpp-vm runs stage 2 to compile src/clpp.clp
/// Stages 1, 2 and 3 must be byte for byte identical.
fn bootstrap() -> bool {
    use std::process::Command;
    let exe = std::env::current_exe().expect("current exe");
    let vm = exe.with_file_name(if cfg!(windows) { "clpp-vm.exe" } else { "clpp-vm" });
    let dir = std::env::temp_dir().join(format!("clpp-bootstrap-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let stage = |n: u32| dir.join(format!("stage{}.clpb", n));
    let mut ok = true;
    for n in 1..=3u32 {
        let started = std::time::Instant::now();
        let status = if n == 1 {
            Command::new(&exe).args(["src/clpp.clp", "src/clpp.clp"]).arg(stage(1)).status()
        } else {
            Command::new(&vm).arg(stage(n - 1)).arg("src/clpp.clp").arg(stage(n)).status()
        };
        if !matches!(status, Ok(s) if s.success()) {
            println!("stage {}: compile failed", n);
            ok = false;
            break;
        }
        let size = std::fs::metadata(stage(n)).map(|m| m.len()).unwrap_or(0);
        let runner = if n == 1 { "clpp-boot" } else { "clpp-vm" };
        print!("stage {}: {} compiled the compiler ({} bytes, {:.1}s)", n, runner, size, started.elapsed().as_secs_f64());
        if n > 1 {
            let same = std::fs::read(stage(n - 1)).ok() == std::fs::read(stage(n)).ok();
            print!("  {}", if same { "identical to the previous stage" } else { "DIFFERENT from the previous stage" });
            ok &= same;
        }
        println!();
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("{}", if ok { "self-hosting: OK" } else { "self-hosting: FAILED" });
    ok
}

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  clpp-boot <file.clp> [args...]\n  clpp-boot test [dir]\n  clpp-boot check <file.clp>...\n  clpp-boot lsp"
    );
    ExitCode::from(2)
}

fn check(files: &[String]) -> bool {
    let loader = |p: &Path| std::fs::read_to_string(p).ok();
    let mut errors = 0;
    for file in files {
        let path = Path::new(file);
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}: cannot read: {}", file, e);
                errors += 1;
                continue;
            }
        };
        let result = analysis::analyze(path, &text, &loader);
        for d in &result.diags {
            let report = diag::Report {
                severity: match d.severity {
                    analysis::Severity::Error => "error",
                    analysis::Severity::Warning => "warning",
                    analysis::Severity::Info => "info",
                },
                message: d.message.clone(),
                file: file.to_string(),
                line: d.line,
                col: d.col,
                len: d.len,
                help: d.help.clone(),
                trace: Vec::new(),
            };
            println!("{}", diag::render(&report, Some(&text)));
            if d.severity == analysis::Severity::Error {
                errors += 1;
            }
        }
    }
    if errors > 0 {
        println!("{} error(s)", errors);
    }
    errors == 0
}

fn real_main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match argv.first().map(String::as_str) {
        None | Some("-h") | Some("--help") => usage(),
        Some("lsp") => ExitCode::from(lsp::run() as u8),
        Some("fmt") => {
            let check_only = argv.iter().any(|a| a == "--check");
            let files: Vec<&String> = argv[1..].iter().filter(|a| *a != "--check").collect();
            if files.is_empty() {
                return usage();
            }
            let mut ok = true;
            for file in files {
                let source = match std::fs::read_to_string(file) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{}: cannot read: {}", file, e);
                        ok = false;
                        continue;
                    }
                };
                match fmt::format(&source, &fmt::Options::default()) {
                    Ok(formatted) if formatted == source => {}
                    Ok(formatted) => {
                        if check_only {
                            println!("{}: not formatted", file);
                            ok = false;
                        } else if let Err(e) = std::fs::write(file, formatted) {
                            eprintln!("{}: cannot write: {}", file, e);
                            ok = false;
                        } else {
                            println!("formatted {}", file);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: not formatted, fix this first: {}", file, e.replace("<source>", file));
                        ok = false;
                    }
                }
            }
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some("bootstrap") => {
            if bootstrap() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some("golden") => {
            let dir = argv.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("tests/golden"));
            if golden(&dir) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some("check") => {
            if argv.len() < 2 {
                return usage();
            }
            if check(&argv[1..]) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some("test") => {
            let dir = argv.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("tests"));
            if run_tests(&dir) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some(file) => match run(Path::new(file), argv[1..].to_vec()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{}", message);
                ExitCode::FAILURE
            }
        },
    }
}

fn main() -> ExitCode {
    // The interpreter recurses on the Rust stack; give it plenty of room.
    let child = std::thread::Builder::new().stack_size(STACK_SIZE).spawn(real_main);
    match child {
        Ok(handle) => handle.join().unwrap_or(ExitCode::FAILURE),
        Err(_) => real_main(),
    }
}
