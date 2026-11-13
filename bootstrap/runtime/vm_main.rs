//! `clpp-vm <file.clpb> [args...]`: runs CL++ bytecode.

mod diag;
mod stdlib;
mod vm;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(file) = argv.first() else {
        eprintln!("usage: clpp-vm <file.clpb> [args...]");
        std::process::exit(2);
    };
    std::process::exit(vm::run_file(file, argv[1..].to_vec()));
}
