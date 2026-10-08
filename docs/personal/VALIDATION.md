# Validation record and display checklist

## Checks completed on this host

These results distinguish compilation, renderer execution and physical display testing. Logs are retained under `/home/hewel/Codes/niri/personal/` outside the source tree.

Host inspection (2026-10-08/09, Asia/Shanghai): CachyOS x86_64, kernel 7.2.9-1-cachyos, Rust/Cargo 1.98.0. Installed libraries included Mesa 26.2.4, libdisplay-info 0.3.0, libinput 1.32, libseat 0.9.3 (seatd), PipeWire 1.6.9, Pango 1.58.2, Wayland 1.26, xkbcommon 1.13.2, Clang 23.1.1 and Vulkan headers/loader 1.4.363. Required pkg-config probes succeeded. Neither source checkout needed a baseline build fix; `Cargo.lock` was unchanged.

| Check | Actual result | Evidence |
| --- | --- | --- |
| Unmodified selected spicy + Smithay build | Passed, `cargo build --locked`, approximately 1m08s | `baseline-build.log` |
| Unmodified workspace tests | Passed: 274 niri and 20 configuration tests, plus workspace/integration/doc tests | `baseline-tests.log` |
| Personal release build | Passed, `cargo build --release --locked`, approximately 1m19s | `release-build.log` |
| Personal workspace tests | Passed: 280 niri, 22 configuration, 1 configuration integration, 2 IPC and 1 IPC doc test | `final-tests.log` |
| Real GLES shader compilation, binding, drawing and readback | Passed with GPU execution required | `glass-pixel-tests.log`, `final-tests.log` |
| Vulkan fallback rendering tests | Passed using the host's default Vulkan ICD | `final-tests.log` |
| Clippy across workspace/all targets | Completed with five existing warnings, no new Glass warnings | `clippy.log` |
| Pinned build helper and example parsing | Passed release build and both example validations | `reproduce-build.log` |
| Independent nested desktop trial | Passed SDR Glass on/off reload, overview/fullscreen and screenshot restoration; final screenshot counts recorded with the trial artifacts | `validation/` |
| Real TTY/DRM HDR output | **Unverified** | Manual steps below |

The final workspace command was:

```sh
NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 cargo test --all --locked
```

GLES execution used Mesa 26.2.4 llvmpipe (LLVM 23.1.1), OpenGL ES 3.2. This is an actual software renderer executing the GLSL, not a claim of physical GPU or HDR display validation. Vulkan's existing rendering/fallback checks used the available default ICD; the complete Vulkan desktop was not tested. A trial specifying a nonexistent lavapipe ICD failed before rendering; removing that invalid override produced the passing final result.

The six targeted Glass tests exercise observable risks: lazy shader compilation with assertions that the Glass program actually drew; neutral Glass matching the ordinary background; HDR PQ readback against an independent conversion expectation at 203 cd/m² reference white; restoration of a custom SDR reference state; translucent input/rounded-edge premultiplication; refraction modes and translated-window geometry; disabled Glass without shader use; Vulkan and non-xray fallback retaining ordinary effects; and real Wayland wallpaper color metadata rejecting HDR input while retaining blur. Existing renderer scenes include overview/zoom. Configuration tests cover defaults, parameter/rule merging, independent `off`/`off false`, invalid values and unknown/ineffective options.

A separate nested release compositor used its own Wayland/IPC sockets and the GTK demo. At 1871×2135, captured SDR screenshots changed 2,387,940 pixels with Glass enabled/disabled and matched exactly after restoring the enabled configuration. Overview and fullscreen toggles completed; an unsupported non-xray request emitted one warning for its category, and the nested compositor exited with status 0. Evidence is in `validation/nested-result.json`, `nested.log` and the PNG captures. This validates ordinary nested SDR rendering and actual visible Glass execution. It does not establish physical HDR output or HDR screenshot/screencast colors.

Initial focused-window captures hid all background effects behind the base's filled focus ring. A reviewer traced this existing policy in the tile/focus-ring code; setting `draw-border-with-background false` in the example resolved it without a renderer change. The GTK demo avoids a separate wallpaper tool and terminal dependency. Foot-specific effect behavior was not revalidated after this configuration correction.

A read-only reviewer inspected the port for renderer and color-path failures; reviewer inspection is not independent test execution. Passing these checks does not establish HDR screenshots, screencasts, direct scanout transitions, perceptual HDR correctness or performance improvements.

## SDR appearance follow-up (2026-10-09)

The default independent example now uses the Glass parameters in the pinned Niri-glass README. `glass-test-preset.kdl` retains the previous validation settings. This follow-up changes examples and opt-in validation tools; production renderer/shader behavior is unchanged.

`scripts/personal-compare-glass.sh` successfully compiled and executed the pinned, unmodified upstream Glass shader and the ported shader with the same actual GLES renderer. Both used one opaque SDR sRGB background capture, identical cached blur (three passes, offset 2), 960×640 output, window position (80,48), window size 640×544, scale 1, effective radius 21, noise 0 and background saturation 1. The original shader's rounding/postprocess helpers were verified identical to its official base; upstream-only padding defaults to zero. No HDR conversion helper was inserted into the reference shader. The comparison omits the foreground window so its opacity/tint cannot mask differences.

| Compared images | Mean absolute RGB error (0–255) | Maximum channel error |
| --- | --- | --- |
| Previous preset vs aligned preset, whole window | 12.2763 | 122 |
| Port vs original shader, aligned preset, whole window | 0.9143 | 119 |
| Same comparison excluding a 48-pixel edge band | 0.0337 | 1 |
| Aligned port with blur vs without blur | 19.6820 | 124 |

The central region is close for this scene; the large maximum errors remain near edges. The port's corrected coordinate mapping and edge/alpha handling are retained, so this does not claim pixel-identical output or perceptual equivalence. The visible old/new difference confirms that preset selection matters. The blur/sharp difference and renderer draw audit verify that the probe actually exercised blur and Glass rather than comparing inactive configurations.

Evidence: `validation/appearance/comparison.html`, `comparison.json`, `measurements.json`, exported `reference.frag`/`ported.frag`, and six PNGs. The records include source revisions, shader hashes, effective configurations and source dirty state. This is a same-scene shader comparison, not execution of the entire original compositor, and does not verify HDR output.

A separate nested release trial switched previous → aligned → previous settings in the same GTK scene. At 1871×2135, old/aligned screenshots differed in 2,385,280 pixels; restoring the previous preset restored the screenshot exactly. It exited with status 0 and used only its own IPC socket. Evidence: `validation/appearance/nested-k27dhh56/` (the binary was the preceding release commit, recorded in `result.json`; renderer code is unchanged by this follow-up). Five existing Glass renderer regression tests passed again. The earlier full workspace results above were not rerun solely for this example/manual-tool change.

## Detected display and limits of this record

The current desktop exposed one connected output: `DP-1`, ViewSonic VG2481-4K, 3840×2160 at 60 Hz, scale 1.75. Its EDID advertises BT.2020 RGB, PQ/ST2084 and HDR static metadata, with desired maximum luminance 400 cd/m², maximum frame-average luminance 400 cd/m² and minimum approximately 0.098 cd/m². These are display declarations, not measurements of actual brightness. The example does not invent a peak-luminance override.

The host has AMD Navi 31 and Raphael graphics. The existing desktop ran on VT 2. No logout, reboot, system compositor replacement or automatic VT switch was performed. Multiple-output behavior remains unverified because only one output was connected.

## Real TTY/DRM trial

Save any work in the existing desktop. Press **Ctrl+Alt+F3**, log in, and first run the independent **SDR** example. This host has a short local symlink to the versioned launcher:

```sh
cd ~/Codes/niri/personal
./tty sdr
# After exiting the SDR trial:
./tty hdr
```

The equivalent portable launcher is `scripts/personal-tty-trial.sh [sdr|hdr]` in the source checkout. It defaults to SDR, refuses to start from a graphical terminal, resolves paths independently of the current directory, and selects the independent artifact or the checkout's release binary. It does not switch VTs or install anything. `hdr` explicitly selects the existing DP-1 HDR example. To create the short link in a freshly prepared sibling layout, run `ln -s niri/scripts/personal-tty-trial.sh tty` from its parent directory.

The expanded SDR command is:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u NIRI_SOCKET -u NOTIFY_SOCKET \
    dbus-run-session \
    /home/hewel/Codes/niri/personal/artifacts/niri-hdr-glass \
    -c /home/hewel/Codes/niri/personal/niri/examples/personal/glass.kdl \
    -- python3 /home/hewel/Codes/niri/personal/niri/examples/personal/demo.py
```

Removing inherited display sockets selects the DRM backend rather than nesting inside the existing desktop. The private D-Bus session avoids sharing a main-session bus. No `--session` is used. If seat permissions or DRM device access prevents launch, retain the error and correct that host/session access issue before interpreting any rendering result.

Exit with **Super+Shift+E**. Repeat the same command with `examples/personal/hdr-dp-1.kdl` to explicitly enable HDR on the detected `DP-1`. On another machine, inspect connector names and EDID first, and change only a trial copy. Exit the trial compositor before returning to the existing desktop with **Ctrl+Alt+F2**. An isolated trial can still exercise display mode changes, so keep its log and restore the original VT after exit.

## Manual observations still required

| Scenario | Procedure | Expected observation |
| --- | --- | --- |
| Ordinary SDR desktop | Run the SDR example; move/resize windows, open overview and toggle fullscreen | Stable wallpaper/window colors and rounded edges; refraction follows the window rather than drifting with its screen position |
| Glass disabled | In a trial copy, add `off` to the Glass block and reload; then use `off false` | Ordinary blur remains; enabling Glass adds refraction/tint; disabling it restores ordinary rendering |
| SDR wallpaper on HDR output | Run the HDR example with the same static background | Glass remains visible; no double conversion, washed-out SDR background, unexplained brightness jump or colored transparent-edge fringe |
| HDR enable/disable | Compare trial copies with `hdr mode="on"` and `mode="off"` | Display mode transitions complete and both modes render correctly; assess real brightness rather than assuming encoded pixels prove it |
| HDR fullscreen entry/exit | In a trial copy use `mode="auto"`; play a known HDR source in a color-management-capable player/game, enter/leave fullscreen | Correct HDR activation and return to SDR; compositor effects/color conversion remain present when composition is required |
| Screenshot colors | Capture SDR, HDR-content and wallpaper/Glass scenes with the custom compositor; inspect exported files in a color-aware viewer | Files have the base compositor's intended color representation without unexpected tint/clipping; compare with the unmodified build if uncertain |
| Screencast colors | Set up the appropriate portal in an isolated test session, select the test output/window, and record SDR and HDR scenes | Captured colors match the base compositor's intended screencast conversion; confirm in a viewer, not just that a stream starts |
| Unsupported non-xray Glass | Use `xray false` with both blur and Glass | One diagnostic for the category; ordinary blur still works; no globally forced xray behavior |
| HDR/linear/wide-gamut wallpaper input | Supply a background client declaring those color spaces | Glass is disabled only for the unsupported background with a deduplicated warning; ordinary effects remain |
| Mixed SDR/HDR outputs | Connect an additional output and move the same window between outputs, including fullscreen transitions | Per-output colors/effects remain correct and no stale cached color-space conversion appears |

The simple demo does not provision a full portal/service environment, generate an HDR video source, or prove screencast correctness. Use known content and a properly configured isolated session for those checks. The display tests above remain unverified until observed on real hardware; do not infer them from the nested trial or shader readback.
