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

The independent example is `examples/personal/smooth-corners.kdl`. The proposed build artifact is `/home/hewel/Codes/niri/personal/artifacts/niri-hdr-glass-smooth`, separate from the existing Glass artifact and installed daily binary. Once that artifact has been built, launch a nested trial from the current desktop:

```sh
/home/hewel/Codes/niri/personal/artifacts/niri-hdr-glass-smooth \
    -c /home/hewel/Codes/niri/personal/niri/examples/personal/smooth-corners.kdl \
    -- python3 /home/hewel/Codes/niri/personal/niri/examples/personal/demo.py
```

No `--session` is needed. The GTK demo provides an SDR wallpaper and window; it needs no swaybg or foot. Edit only a trial configuration, not the active desktop configuration. `Mod+F` toggles fullscreen, `Mod+O` toggles overview and `Mod+Shift+E` exits the trial; Mod defaults to Alt when nested and Super on the TTY backend.

For the separate real TTY trial, the console helper's smooth mode is `./tty smooth` from `/home/hewel/Codes/niri/personal/`. Follow the VT access and return instructions in [VALIDATION.md](VALIDATION.md): use an unused TTY, exit the trial compositor, then return to the existing desktop's VT. Do not infer physical HDR validation from a nested SDR window.

The existing daily/GDM installation predates this extension. Its earlier validation does not validate smooth corners, and this trial does not replace its binary, configuration or services.

## Validation and acceptance

Smooth-corner build, parser, renderer and reference-pixel checks are being completed separately from the original HDR + Glass validation. Until their final results are recorded, the original [VALIDATION.md](VALIDATION.md) does not establish that these new paths passed. No performance improvement is claimed.

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
