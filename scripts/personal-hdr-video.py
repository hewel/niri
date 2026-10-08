#!/usr/bin/env python3
"""Explicit HDR10 console trial with per-run compositor/player logs and IPC readback."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time

SOURCE = Path(__file__).resolve().parent.parent
VALIDATION = SOURCE.parent / "validation/hdr-video"


def write_config(run, mode):
    if mode not in ("auto", "on", "off"):
        raise ValueError("invalid HDR mode")
    # This spicy version disables HDR by omitting the hdr node, not mode="off".
    output = '\noutput "DP-1" {' + ('' if mode == 'off' else ' hdr mode="' + mode + '";') + ' }\n'
    config = ('include ' + json.dumps(str(SOURCE / "examples/personal/glass.kdl"))
              + output
              + 'window-rule { match app-id="^personal-hdr-video$"; open-floating true; }\n')
    temporary = run / "trial.kdl.tmp"
    temporary.write_text(config)
    temporary.replace(run / "trial.kdl")
    with (run / "modes.jsonl").open("a") as log:
        log.write(json.dumps({"time": time.time(), "mode": mode}) + "\n")


def get_property(ipc, name):
    with socket.socket(socket.AF_UNIX) as connection:
        connection.settimeout(1)
        connection.connect(str(ipc))
        connection.sendall((json.dumps({"command": ["get_property", name], "request_id": 1}) + "\n").encode())
        with connection.makefile("r") as response:
            for line in response:
                value = json.loads(line)
                if value.get("request_id") == 1:
                    return value
    raise RuntimeError("player IPC closed")


def startup(run):
    record = json.loads((run / "run.json").read_text())
    if not os.environ.get("WAYLAND_DISPLAY") or not os.environ.get("NIRI_SOCKET"):
        raise RuntimeError("startup must be launched by the trial compositor")
    ipc = run / "mpv.sock"
    # Player Vulkan is independent of the compositor's default GLES renderer.
    arguments = ["mpv", "--no-config", "--vo=gpu-next", "--gpu-api=vulkan",
                 "--gpu-context=waylandvk", "--hwdec=auto-safe",
                 "--target-colorspace-hint=yes", "--target-colorspace-hint-mode=source",
                 "--wayland-app-id=personal-hdr-video", "--keep-open=yes",
                 "--mute=yes", f"--input-ipc-server={ipc}",
                 f"--log-file={run / 'mpv.log'}", "--msg-level=all=v",
                 f"--script={run / 'keys.lua'}", "--", record["media"]]
    (run / "player-arguments.json").write_text(json.dumps(arguments, indent=2) + "\n")
    with (run / "demo.log").open("w") as demo_log, (run / "player-console.log").open("w") as player_log:
        demo = subprocess.Popen(["python3", str(SOURCE / "examples/personal/demo.py")],
                                stdout=demo_log, stderr=demo_log)
        player = subprocess.Popen(arguments, stdout=player_log, stderr=player_log)
        try:
            with (run / "player-state.jsonl").open("w") as log:
                while player.poll() is None:
                    if ipc.exists():
                        state = {"time": time.time()}
                        try:
                            for name in ("fullscreen", "video-params", "video-target-params", "hwdec-current", "frame-drop-count", "decoder-frame-drop-count"):
                                state[name] = get_property(ipc, name)
                        except (OSError, ValueError, RuntimeError) as error:
                            state["readback_error"] = str(error)
                        log.write(json.dumps(state) + "\n")
                        log.flush()
                    time.sleep(1)
            (run / "player-exit.json").write_text(json.dumps({"exit_code": player.returncode}) + "\n")
        finally:
            for child in (player, demo):
                if child.poll() is None:
                    child.terminate()
                    child.wait(timeout=10)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=SOURCE.parent / "artifacts/niri-hdr-glass")
    parser.add_argument("--media", type=Path)
    parser.add_argument("--startup", type=Path)
    parser.add_argument("--set-mode", choices=("auto", "on", "off"))
    parser.add_argument("--run-dir", type=Path)
    parser.add_argument("--prepare-only", action="store_true", help="write trial files without starting a compositor")
    args = parser.parse_args()
    if args.startup and args.set_mode:
        parser.error("startup and mode switching are separate operations")
    if args.set_mode and not args.run_dir:
        parser.error("mode switching requires --run-dir")
    if args.startup or args.set_mode:
        run = (args.startup or args.run_dir).resolve()
        if run.parent != VALIDATION.resolve() or not (run / "run.json").is_file():
            parser.error("run directory must be a prepared trial under validation/hdr-video")
        if args.startup:
            startup(run)
        else:
            write_config(run, args.set_mode)
        return
    console = subprocess.run(["tty"], capture_output=True, text=True).stdout.strip()
    if not args.prepare_only and (not console.startswith("/dev/tty") or not console[8:].isdigit()):
        parser.error("run from a real TTY, or use ./tty video after Ctrl+Alt+F3")
    media = args.media or Path((VALIDATION / "hdr-path.txt").read_text().strip())
    media = media.resolve(strict=True)
    probe = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-select_streams", "v:0",
                                               "-show_streams", "-of", "json", str(media)], text=True))
    stream = probe["streams"][0]
    if stream.get("color_transfer") != "smpte2084" or stream.get("color_primaries") != "bt2020":
        parser.error("selected media must be tagged PQ/BT.2020 HDR")
    binary = args.binary.resolve(strict=True)
    VALIDATION.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="tty-", dir=VALIDATION))
    record = {"media": str(media), "probe": probe, "binary": str(binary),
              "binary_version": subprocess.check_output([str(binary), "--version"], text=True).strip(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "console": console, "acceptance": "pending human observation"}
    (run / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    write_config(run, "auto")
    commands = []
    for key, mode in (("F1", "on"), ("F2", "auto"), ("F3", "off")):
        argv = ["python3", str(Path(__file__).resolve()), "--set-mode", mode, "--run-dir", str(run)]
        commands.append('mp.add_key_binding(' + json.dumps(key) + ',' + json.dumps("trial-" + mode)
                        + ', function() mp.commandv("run", ' + ', '.join(map(json.dumps, argv))
                        + '); mp.osd_message(' + json.dumps("Requested HDR mode: " + mode) + ') end)')
    (run / "keys.lua").write_text("\n".join(commands) + "\n")
    if args.prepare_only:
        print(run)
        return
    env = os.environ.copy()
    for name in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET", "NIRI_SOCKET", "NOTIFY_SOCKET"):
        env.pop(name, None)
    env["RUST_LOG"] = "niri=debug"
    env["NIRI_DISABLE_SYSTEM_MANAGER_NOTIFY"] = "1"
    command = ["dbus-run-session", str(binary), "-c", str(run / "trial.kdl"), "--",
               "python3", str(Path(__file__).resolve()), "--startup", str(run)]
    print(f"Logs: {run}\nf: fullscreen; F1: HDR on; F2: auto; F3: off; Shift+i: statistics.\n"
          "Exit trial: Super+Shift+E; return to desktop: Ctrl+Alt+F2.", flush=True)
    with (run / "niri.log").open("w") as log:
        result = subprocess.run(command, env=env, stdout=log, stderr=log)
    (run / "compositor-exit.json").write_text(json.dumps({"exit_code": result.returncode}) + "\n")
    print(f"Trial exited ({result.returncode}). Logs: {run}")
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
