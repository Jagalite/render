"""Run reproducible offline foundation checks and retain exact commands/results.

GPU/browser execution is separate because it needs a graphics device and an
isolated browser. This script never updates committed evidence automatically.
"""
import datetime
import json
import os
from pathlib import Path
import subprocess
import sys
import time

root = Path("artifacts/evidence")
logs = root / "logs"
logs.mkdir(parents=True, exist_ok=True)
checks = [
    ("format", ["cargo", "fmt", "--all", "--check"]),
    ("native_clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"]),
    ("native_tests", ["cargo", "test", "--workspace", "--locked", "--offline"]),
    ("wasm_clippy", ["cargo", "clippy", "-p", "render-web", "--target", "wasm32-unknown-unknown", "--locked", "--offline", "--", "-D", "warnings"]),
    ("wasm_tests", ["cargo", "test", "-p", "render-core", "--target", "wasm32-unknown-unknown", "--locked", "--offline"]),
    ("linux_check", ["cargo", "check", "-p", "render-host", "--target", "x86_64-unknown-linux-gnu", "--locked", "--offline"]),
    ("windows_check", ["cargo", "check", "-p", "render-host", "--target", "x86_64-pc-windows-msvc", "--locked", "--offline"]),
    ("native_build", ["cargo", "build", "-p", "render-host", "-p", "render-ffi", "--locked", "--offline"]),
    ("abi_header", ["python3", "scripts/generate-abi-header.py", "--check"]),
    ("independent_ffi", ["python3", "scripts/ffi-conformance.py"]),
    ("web_build", ["sh", "scripts/build-web.sh"]),
    ("dependency_audit", ["python3", "scripts/dependency-audit.py"]),
    ("benchmark_build", ["cargo", "build", "--release", "-p", "render-host", "--example", "m00", "--locked", "--offline"]),
    ("representation_benchmark", ["/usr/bin/time", "-l", "target/release/examples/m00"]),
]
env = dict(os.environ, CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-bindgen-test-runner")
report = {"started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(), "checks": [],
          "scope": "Native macOS runtime; WASM core in Node; Linux/Windows compile checks only. GPU and browser reports are separate."}
if sys.argv[1:] == ["--resume"]:
    report = json.loads((root / "validation_report.json").read_text())
elif sys.argv[1:]:
    raise SystemExit("usage: validate-foundation.py [--resume]")
for name, command in checks:
    previous = next((c for c in report["checks"] if c["name"] == name), None)
    if previous and previous["exit_code"] == 0 and previous["command"] == command:
        continue
    start = time.monotonic()
    result = subprocess.run(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (logs / (name + ".txt")).write_text(result.stdout)
    report["checks"] = [c for c in report["checks"] if c["name"] != name]
    report["checks"].append({"name": name, "command": command, "exit_code": result.returncode,
                             "seconds": time.monotonic() - start, "log": "logs/" + name + ".txt"})
    report["status"] = "passed" if all(c["exit_code"] == 0 for c in report["checks"]) else "failed"
    (root / "validation_report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"{name}: exit {result.returncode}", flush=True)
    if result.returncode:
        print(result.stdout[-6000:], flush=True)
        raise SystemExit(result.returncode)
