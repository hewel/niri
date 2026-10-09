# Optional smooth corners

This extension adds a smoothing property to the existing corner-radius setting. It adapts Lisse's Figma squircle construction: cubic shoulders join a circular arc rather than replacing the rounded rectangle with a superellipse. The source is [JaceThings/Lisse](https://github.com/JaceThings/Lisse), commit `173846978ed806e14c78154ec914a9645a991932`, particularly `packages/core/src/corner-params.ts`. Its MIT notice is retained in [LICENSE-Lisse](LICENSE-Lisse), and the revision is recorded as `LISSE_REV` in [versions.env](versions.env).

This is a port of that corner construction into niri's geometry and shader paths. It does not implement every Lisse algorithm, reproduce Apple's native continuous corners, or promise exact G2 continuity. Niri retains its CSS-style radius fitting and distributes available edge space proportionally between neighboring corners. Lisse's full path builder uses a different priority/budget policy and has special capsule branches; results can therefore differ for large or asymmetric radii and constrained geometry.

## Configuration and scope

```kdl
window-rule {
    match app-id="^personal-glass-demo$"
    geometry-corner-radius 20 smoothing=0.65
    clip-to-geometry true
}
```

`smoothing` accepts finite numbers from 0 to 1. Omission or `smoothing=0` preserves circular-corner behavior. The existing one-radius syntax and four-radius order (top-left, top-right, bottom-right, bottom-left) remain supported:

```kdl
geometry-corner-radius 10 20 30 40 smoothing=0.65
```

Zero-radius corners remain square. A later matching rule replacing `geometry-corner-radius` replaces its smoothing too; omitting smoothing in that replacement resets it to zero. To restore the original shape, use the original radii without the property or explicitly set `smoothing=0`.

The shared shape is applied to window content clipping, backgrounds, borders/focus rings, shadows, resize rendering and Glass's signed-distance calculations. Set `clip-to-geometry true` to clip window content. Ordinary smooth corners remain available on GLES and Vulkan; Liquid Glass itself retains the existing GLES-only restrictions described in [README.md](README.md).

Layer rules retain their original meaning: `geometry-corner-radius` affects existing layer background/shadow geometry, without adding layer-content clipping. Single-window screenshots and single-window screencasts retain the base capture path; this extension does not add corner clipping to those captures. Whole-output captures can show the composed corners. Neither observation proves HDR screenshot or screencast color correctness.

## Separate trial

The independent example is `examples/personal/smooth-corners.kdl`. The release build artifact is `/home/hewel/Codes/niri/personal/artifacts/niri-hdr-glass-smooth`, separate from the existing Glass artifact and installed daily binary. Launch a nested trial from the current desktop:

```sh
/home/hewel/Codes/niri/personal/artifacts/niri-hdr-glass-smooth \
    -c /home/hewel/Codes/niri/personal/niri/examples/personal/smooth-corners.kdl \
    -- python3 /home/hewel/Codes/niri/personal/niri/examples/personal/demo.py
```

No `--session` is needed. The GTK demo provides an SDR wallpaper and window; it needs no swaybg or foot. Edit only a trial configuration, not the active desktop configuration. `Mod+F` toggles fullscreen, `Mod+O` toggles overview and `Mod+Shift+E` exits the trial; Mod defaults to Alt when nested and Super on the TTY backend.

For the separate real TTY trial, run `./tty smooth` from `/home/hewel/Codes/niri/personal/` for SDR. `./tty smooth-hdr` explicitly selects `smooth-hdr-dp-1.kdl`, enabling HDR only on the previously detected PQ-capable `DP-1` display, using its EDID rather than a peak override. Recheck the connector and capabilities on other machines. Follow the VT access and return instructions in [VALIDATION.md](VALIDATION.md): use an unused TTY, exit the trial compositor, then return to the existing desktop's VT. Do not infer physical HDR validation from a nested SDR window.

The existing daily/GDM installation predates this extension. Its earlier validation does not validate smooth corners, and this trial does not replace its binary, configuration or services.

## Validation and acceptance

The initial independent release artifact was built from `88ff7ceff854562045eed0e849c6890745d15a04` (rendering implementation `ffc387fb4ac7a4a55d3c2402fd94f41f7052996c`). Its SHA256 is `0372bb7c8b5b0fe1d9368b63760b1ffb23de49df32c601513aed88d8e25dc606`. That initial binary is preserved as `artifacts/niri-hdr-glass-smooth-88ff7cef`; the inner-edge correction below changes the compositor, and the current trial binary version/hash is in `artifacts/smooth-corners-build-record.json`. The original accepted daily artifact and installed binary still have SHA256 `a577c86e90d4dda3727d935b70b03ccc8c7c693cbe0a6ee71ad0ea3af7aa386a`.

Initial-version validation on this host:

- `cargo test --locked --workspace`, with `NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1`: 283 niri tests, 27 configuration tests, one configuration integration test, two IPC tests and one IPC doctest passed. The manual Glass reference probe was excluded from that command, then executed separately and passed.
- The renderer checks include actual GLES and Vulkan shader compilation, uniform/texture binding and drawing, including smooth border/shadow, framebuffer clipping, resize and representative custom resize shaders in SDR and HDR blend states. The software GLES pixel probe is distinct from physical HDR display validation.
- Five reference masks were generated by compiling the pinned Lisse core without source changes and rasterizing its SVG paths with Cairo. Their 11 PNG/SVG/metadata artifacts reproduced byte-for-byte. GLES alpha differences have maximum 33–39 and full-image means 0.0640–0.2233 on the 0–255 scale. These small edge differences reflect the retained niri smoothstep versus Cairo coverage antialiasing; this is not pixel identity.
- Neutral Glass matches the corresponding filled mask byte-for-byte in SDR/HDR at scales 1 and 1.75. The base background path's one-logical-pixel radius expansion to suppress bleed is retained explicitly. Hard shadow uses the same mask; the tests also cover thick inset borders and overlarge shadow radii that previously could produce invalid cubic budgets.
- With smoothing disabled, the pinned Glass appearance comparison reproduces the previous measurements exactly, including interior maximum RGB difference 1 and mean 0.03371/255 against the original Glass source. Existing circular clipping and shadow branches remain intact.
- The release build and all five example configurations validate successfully. Nested Wayland trial `smooth-nested-ps6vyqw8` drew on `/dev/dri/renderD128` (`amdgpu`) at scale 1.75, exercised circular/smooth reloads, asymmetric radii, resizing, fullscreen entry/exit and overview, and exited with code 0. Five successful reloads and nine screenshots are recorded without config/shader errors. The initial helper run was rejected because flattening includes created duplicate `binds`; the helper now preserves includes, validates every generated config, and rejects runtime config failures.
- Independent read-only review identified the inset/negative-spread radius-budget risk; shader fitting and shadow fitting address it. Projection retains its best seed/endpoints when Newton refinement would regress. The review's mathematical sampling is separate from the implementer's GPU tests.

Evidence is stored under `/home/hewel/Codes/niri/personal/validation/smooth-corners/`, including `workspace-tests.log`, `release-build.log`, `measurements.json`, `comparison.html`, the compatibility comparison and nested screenshots/logs. The interactive comparison page was also checked in Chrome with all five image pairs and slider endpoints. A separate `artifacts/smooth-corners-build-record.json` records the binary, source pins and acceptance limits.

Physical TTY/DRM HDR output and human visual acceptance remain **unverified for this new build**. The original [VALIDATION.md](VALIDATION.md) and earlier HDR10 acceptance apply to the previous daily binary. No performance improvement is claimed.

### Inner-edge antialiasing correction

The user subsequently passed the manual trial and reported slight jaggedness on the inside of rounded borders. DP-1's currently observed scale is 1.75. The correction preserves the contour and color conversion, but replaces the smoothed shape's fixed one-physical-pixel smoothstep with analytic coverage of a square pixel against the local boundary normal. This accounts for the wider footprint of diagonal edges. The inset mask is also evaluated immediately outside the inner rectangle, rather than truncating its AA coverage with a hard bounds check. Circular (`smoothing=0`) behavior is unchanged.

A new regression check draws a 2px border with the compositor's 0.5-logical-pixel corner allowance at scale 1.75. Its independent reference uses the unmodified pinned Lisse outer and inner paths, rasterized at 8x then box-filtered. Before the fix the maximum alpha error was 27/255 and full-image mean 0.24048/255, failing the coverage check. After the fix they are 3/255 and 0.0112/255. All 14 reference SVG/PNG/metadata fixtures reproduce exactly; fixture regeneration now additionally requires Python Pillow.

Fresh workspace tests pass with 284 niri tests, 27 configuration tests and the existing integration/IPC checks. Real GLES/Vulkan drawing and SDR/HDR mask consistency remain covered. Independent read-only review found a subpixel-rectangle coverage edge case; exact interval intersection now caps that coverage correctly. The base renderer's existing inner/outer mask composition is retained. No additional captures or rendering passes are introduced.

Evidence for this correction is in `validation/corner-aa/`, including the failing/passing pixel logs, before/after border PNGs, the independent reference and workspace test log. The same `./tty smooth` and `./tty smooth-hdr` commands select the rebuilt independent trial artifact. The installed daily binary and active configuration are preserved. Human confirmation of the corrected inner edge remains pending; the reported acceptance above applies to the initial binary.

To reproduce the pixel/reference checks (Lisse and its JS tooling are required only when regenerating fixtures):

```sh
node scripts/personal-corner-reference.mjs ../Lisse ../validation/smooth-corners/reference
NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 \
    NIRI_CORNER_COMPARE_DIR=../validation/smooth-corners \
    cargo test --locked --lib smooth_corners -- --nocapture
python3 scripts/personal-smooth-trial.py --output-root ../validation/smooth-corners
```

The reference helper requires Node, TypeScript (`tsc`, or an explicit `NIRI_TSC` executable), `rsvg-convert`, and Python Pillow. It verifies the exact clean Lisse revision before compiling. It never fetches an arbitrary tip. The test PNGs are committed, so ordinary compositor builds/tests do not need that checkout or tooling.

Manual acceptance still needs the following observations:

| Check | Expected observation |
| --- | --- |
| Compare `smoothing=0` and `0.65` at the same radius | Zero restores circular corners; the enabled shape has smooth shoulders without moving the straight edges |
| Resize and asymmetric radii | No overlapping corners, clipped shoulders or drifting background/refraction; square corners remain square |
| Fullscreen entry/exit and overview | Correct transition between the base fullscreen policy and the configured window shape; no stale clipping after returning |
| Live reload and disable/restore | Shape changes apply coherently to content, background, border and shadow; disabling restores the original geometry |
| Thin border against a bright background | No visible seam between content/background clipping and the border or Glass edge; assess fractional scaling too |
| Real TTY HDR and captures | Verify real display rendering and screenshot/screencast colors using the existing HDR checklist; these remain device acceptance work |

Use whole-output screenshots to inspect the composed geometry. Treat single-window capture behavior separately, and retain artifacts/logs with the exact build commit. The Lisse revision is an attribution/reproduction record; building the compositor does not require a neighboring Lisse checkout or its JavaScript dependencies.
