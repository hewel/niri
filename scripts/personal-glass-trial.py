#!/usr/bin/env python3
"""Compare presets in one isolated nested compositor; never addresses the parent IPC."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


def main():
    source = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=source / "target/release/niri")
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args()
    if not os.environ.get("WAYLAND_DISPLAY"):
        raise SystemExit("Run from an existing Wayland desktop for a nested trial.")
    binary = args.binary.resolve()
    args.output_root.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="nested-", dir=args.output_root.resolve()))
    aligned = (source / "examples/personal/glass.kdl").read_text() + "\nanimations { off; }\n"
    previous = aligned + (source / "examples/personal/glass-test-preset.kdl").read_text().split('include "glass.kdl"', 1)[1]
    config = output / "trial.kdl"
    config.write_text(previous)
    env = os.environ.copy()
    parent_socket = env.pop("NIRI_SOCKET", "")
    env.pop("NOTIFY_SOCKET", None)
    env["NIRI_DISABLE_SYSTEM_MANAGER_NOTIFY"] = "1"
    env["RUST_LOG"] = "niri=debug"
    log_path = output / "nested.log"
    result = {"binary_version": subprocess.check_output([str(binary), "--version"], text=True).strip()}
    with log_path.open("w") as log:
        child = subprocess.Popen([str(binary), "-c", str(config), "--", "python3",
                                  str(source / "examples/personal/demo.py")], env=env, stdout=log, stderr=log)
        ipc_env = None

        def command(*arguments):
            assert ipc_env and ipc_env["NIRI_SOCKET"].endswith(f".{child.pid}.sock")
            return subprocess.check_output([str(binary), "msg", *arguments], env=ipc_env,
                                           text=True, stderr=subprocess.STDOUT, timeout=10)

        def screenshot(name):
            dest = output / f"{name}.png"
            command("action", "screenshot-screen", "--show-pointer", "false", "--path", str(dest))
            for _ in range(100):
                if dest.exists() and dest.stat().st_size > 0:
                    return
                time.sleep(.1)
            raise RuntimeError(f"Screenshot not saved: {dest}")

        try:
            for _ in range(200):
                if child.poll() is not None:
                    raise RuntimeError(f"Nested compositor exited early; see {log_path}")
                match = re.search(r"IPC listening on: (\S+)", log_path.read_text())
                if match:
                    socket = match.group(1)
                    assert socket != parent_socket and socket.endswith(f".{child.pid}.sock")
                    ipc_env = env.copy()
                    ipc_env["NIRI_SOCKET"] = socket
                    ipc_env.pop("DISPLAY", None)
                    break
                time.sleep(.1)
            else:
                raise RuntimeError("Nested IPC socket unavailable")
            for _ in range(100):
                windows = json.loads(command("--json", "windows"))
                if any(w.get("app_id") == "personal-glass-demo" for w in windows):
                    break
                time.sleep(.1)
            else:
                raise RuntimeError("GTK demo window did not map")
            result.update(pid=child.pid, socket=socket, windows=windows,
                          outputs=json.loads(command("--json", "outputs")),
                          layers=json.loads(command("--json", "layers")))
            time.sleep(2)
            screenshot("old-preset")
            config.write_text(aligned)
            time.sleep(2)
            screenshot("upstream-preset")
            config.write_text(previous)
            time.sleep(2)
            screenshot("old-restored")
            try:
                command("action", "quit", "--skip-confirmation")
            except subprocess.CalledProcessError:
                if "quitting as requested" not in log_path.read_text():
                    raise
            child.wait(timeout=10)
            result["exit_code"] = child.returncode
            assert child.returncode == 0
        finally:
            if child.poll() is None:
                child.terminate()
                child.wait(timeout=10)
            (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(output)


if __name__ == "__main__":
    main()
