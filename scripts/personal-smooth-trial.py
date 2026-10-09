#!/usr/bin/env python3
"""Compare corner curves in one isolated nested compositor; never addresses the parent IPC."""
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
    parser.add_argument("--binary", type=Path, default=source.parent / "artifacts/niri-hdr-glass-smooth")
    parser.add_argument("--output-root", type=Path, required=True)
    args = parser.parse_args()
    if not os.environ.get("WAYLAND_DISPLAY"):
        raise SystemExit("Run from an existing Wayland desktop for a nested trial.")
    binary = args.binary.resolve()
    args.output_root.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="smooth-nested-", dir=args.output_root.resolve()))
    (output / "glass-base.kdl").write_text((source / "examples/personal/glass.kdl").read_text())
    base = (source / "examples/personal/smooth-corners.kdl").read_text().replace('include "glass.kdl"', 'include "glass-base.kdl"') + "\nanimations { off; }\n"
    def curve(radii="32", smoothing="0.65"):
        return base + f'\nwindow-rule {{ match app-id="^personal-glass-demo$"; geometry-corner-radius {radii} smoothing={smoothing}; }}\n'
    config = output / "trial.kdl"
    def write_config(text):
        config.write_text(text)
        subprocess.run([str(binary), "validate", "-c", str(config)], check=True, capture_output=True, text=True)
    write_config(curve(smoothing="0"))
    env = os.environ.copy()
    parent_socket = env.pop("NIRI_SOCKET", "")
    env.pop("NOTIFY_SOCKET", None)
    env["NIRI_DISABLE_SYSTEM_MANAGER_NOTIFY"] = "1"
    env["RUST_LOG"] = "niri=debug,niri_config=debug"
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
            result["states"] = []
            def capture(name, text=None):
                if text is not None:
                    write_config(text)
                    (output / f"{name}.kdl").write_text(text)
                time.sleep(1.5)
                assert "error loading config" not in log_path.read_text(), f"Config failed during trial; see {log_path}"
                screenshot(name)
                result["states"].append({"name": name, "windows": json.loads(command("--json", "windows"))})
            capture("circular", curve(smoothing="0"))
            capture("smooth", curve())
            command("action", "set-column-width", "45%")
            capture("resized")
            capture("asymmetric", curve("48 24 0 32"))
            command("action", "fullscreen-window")
            capture("fullscreen")
            command("action", "fullscreen-window")
            capture("fullscreen-exit")
            command("action", "toggle-overview")
            capture("overview")
            command("action", "toggle-overview")
            capture("smooth-restored", curve())
            capture("circular-restored", curve(smoothing="0"))
            try:
                command("action", "quit", "--skip-confirmation")
            except subprocess.CalledProcessError:
                if "quitting as requested" not in log_path.read_text():
                    raise
            child.wait(timeout=10)
            result["exit_code"] = child.returncode
            result["config_reload_count"] = max(0, log_path.read_text().count(f'loaded config from "{config}"') - 1)
            assert child.returncode == 0
        finally:
            if child.poll() is None:
                child.terminate()
                child.wait(timeout=10)
            (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(output)


if __name__ == "__main__":
    main()
