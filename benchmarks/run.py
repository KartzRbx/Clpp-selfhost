"""Runs the benchmarks in the interpreter and on the VM.

    python benchmarks/run.py            run, print a table and save results.json
    python benchmarks/run.py --runs 5   more runs per benchmark (median is kept)

Each program must print the same thing in both runtimes. The VM time is only
the run of the compiled .clpb; compiling is measured separately.
"""

import json
import pathlib
import platform
import statistics
import subprocess
import sys
import tempfile
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
EXE = ".exe" if sys.platform == "win32" else ""
BOOT = ROOT / "bootstrap" / "target" / "release" / f"clpp-boot{EXE}"
VM = ROOT / "bootstrap" / "target" / "release" / f"clpp-vm{EXE}"


def timed(cmd):
    start = time.perf_counter()
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
    elapsed = time.perf_counter() - start
    if result.returncode != 0:
        sys.exit(f"failed: {' '.join(map(str, cmd))}\n{result.stderr}")
    return elapsed, result.stdout.strip()


def main():
    runs = int(sys.argv[sys.argv.index("--runs") + 1]) if "--runs" in sys.argv else 3
    for exe in (BOOT, VM):
        if not exe.exists():
            sys.exit(f"build first: cargo build --release --manifest-path bootstrap/Cargo.toml ({exe} missing)")
    results = []
    with tempfile.TemporaryDirectory() as tmp:
        for program in sorted(HERE.glob("*.clp")):
            bytecode = pathlib.Path(tmp) / (program.stem + ".clpb")
            compile_time, _ = timed([BOOT, "build", program.relative_to(ROOT), "-o", bytecode])
            interp_times, vm_times = [], []
            interp_out = vm_out = ""
            for _ in range(runs):
                t, interp_out = timed([BOOT, program.relative_to(ROOT)])
                interp_times.append(t)
                t, vm_out = timed([VM, bytecode])
                vm_times.append(t)
            if interp_out != vm_out:
                sys.exit(f"{program.name}: interpreter and VM disagree:\n{interp_out}\n---\n{vm_out}")
            first_line = program.read_text(encoding="utf-8").splitlines()[0].lstrip("< ").strip()
            row = {
                "name": program.stem,
                "what": first_line,
                "output": interp_out,
                "interpreter": round(statistics.median(interp_times), 3),
                "vm": round(statistics.median(vm_times), 3),
                "compile": round(compile_time, 3),
            }
            row["speedup"] = round(row["interpreter"] / row["vm"], 1) if row["vm"] > 0 else None
            results.append(row)
            print(f"{row['name']:<14} interpreter {row['interpreter']:>7.3f}s   vm {row['vm']:>7.3f}s   x{row['speedup']}")
    machine = f"{platform.system()} {platform.machine()}, {platform.processor() or 'unknown CPU'}"
    data = {"machine": machine, "runs": runs, "date": time.strftime("%Y-%m-%d"), "results": results}
    (HERE / "results.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"saved benchmarks/results.json ({machine})")


if __name__ == "__main__":
    main()
