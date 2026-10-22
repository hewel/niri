# Personal changes: ownership and regression checks

The first three refactors start after `a19074380b26899b67e8c9dbcc22aa315a3fdea4`.
They keep the spicy and Smithay revisions in [versions.env](versions.env), the public
KDL settings, and the accepted visual behavior. They do not widen Glass support.

The implementation commits, in order:

1. `92fd1aa53cc68754ceebe4bccdf122c951fdb33b` — daily deployment lifecycle.
2. `1fe2ba503bb0c30dd576c84e8857d18f3f855f31` — Glass preparation and program pairing.
3. `5617c29e53cf055bd7e75ce77e33693343812cd1` — shared corner contours and shadow references.

| Responsibility | Owner | Caller contract |
| --- | --- | --- |
| Daily installation and rollback | `scripts/personal-deploy.py` | Explicit accepted artifact; stage and validate those bytes, publish a complete immutable bundle, then atomically select it for the next login |
| Session startup | Stable launcher plus bundle's `session.sh` | Pin one bundle through migration, validation and systemd startup; do not change an active desktop |
| Glass eligibility and background preparation | `render_helpers/liquid_glass.rs` | Resolve a request, prepare its background, return a shader with matching parameters or preserve ordinary fallback |
| Xray placement, target isolation and scene lifetime | `render_helpers/xray.rs` | Populate through a frame scope; request capture-safe snapshot routing without accessing target buffers |
| SDR capture color operation | `render_helpers/blend.rs` | Scope scene preparation and buffer writes together; restore normal frame state on success or returned error |
| Window unmap | `State::unmap_window` in `niri.rs` | Identify null-buffer removal or toplevel destruction; preserve their distinct snapshot and remap ordering |
| Smooth corner contour | `shaders/rounding_alpha.frag` | Own fitted geometry, cubic/arc intersections and row spans; shadow owns Gaussian integration |

## Daily deployment

[DAILY.md](DAILY.md) defines install/status/rollback usage and the distinction between
accepted, running, daily and rollback builds. [GLOSSARY.md](../../GLOSSARY.md) records those terms.
Configuration migration remains a separate one-time operation; deployment does not
restore old configuration on rollback. Incompatible rollback is refused.

The ordinary user validates a staged copy of the accepted executable. A separately
passed plan digest binds the privileged publication to the checked manifest. The
privileged adapter copies and verifies files into fixed destinations and never runs
the staged executable as root. A store lock and expected selection check serialize
updates. Incomplete unpublished copies can be retried; old complete bundles remain.

The stable session entry pins the bundle before invoking its helper and imports that
pin into systemd. The first upgrade from a flat installation retains a hash-pinned
flat executable/wrapper so an already executing old login can finish its handoff.
This transition does not select a later bundle implicitly.

## Glass

`GlassRequest` rejects `off` and explicit non-xray settings before visibility is
computed. Renderer rejection happens before rendering; removing Glass does not
remove explicit xray, blur, noise or saturation. `PreparedBackground` owns shader
availability, committed input metadata checks, capture preparation, and ordinary
fallback. Its private representation pairs Glass parameters with a GLES program.
Xray elements no longer read a shader cache independently of those parameters.

`EffectBuffer` still owns capture/cache lifetime and scoped SDR capture. Eligibility
uses committed color metadata, including the retained metadata for cached captures,
not pixel magnitude. GLES Glass remains limited to SDR sRGB xray input. Draw-time
output conversion still comes from the existing `FrameBlendState`; framebuffer and
Vulkan behavior are retained. A disabled request does not compile Glass or create a
Glass-specific pass. No new continuous redraw, capture or allocation is introduced
for disabled Glass.

## Smooth contours

`niri_smooth_contour` fits radii and computes corner budgets/extents once. Both
distance sampling and soft shadows use it. `niri_smooth_row` hides corner selection
and cubic/arc intersection; shadow samples the row span in its existing four-step
Gaussian integration. No contour preparation was moved into that sample loop.
Circular corners, projection, normals, pixel-area border antialiasing, and output
color conversion retain their existing formulas.

Six soft-shadow PNG references record the rendering before extraction. Their
`source.json` explicitly identifies them as regression references, not an independent
proof of the Gaussian approximation. Independent pinned Lisse path tests remain.

## Xray frame and snapshot target

The next three refactors start after `1a2305913fb59cafab15ee7d689627c05ce8a7f8`.
They retain the pinned Smithay, visual settings and protocol behavior.

The Xray module owns its per-target background/backdrop buffers. `XrayFrame` ties
temporary layer elements to one render or snapshot operation; dropping the frame
clears those elements across every target but retains prepared textures, presentation
feedback and allocated capacity. Niri supplies the scene without owning buffer layout.

Closing snapshots use `with_snapshot_background` to select a capture-safe background
independently of the window's own capture target. The route is restored on scope exit,
including errors and unwinding. Tile no longer swaps buffer pointers, and emitted
elements retain the selected buffer after routing is restored. Capture-blocked layers
must remain blocked when sampled through effects and closing animations.

## SDR capture operation

`with_sdr_capture` covers renderer-dependent scene and pointer preparation through
the final capture write. Screenshot selection scopes each source output separately;
screen/window screenshots, screencopy, image-copy capture and PipeWire use the same
operation. Privacy targets remain independent of color state and storage format.

GLES, Vulkan and TTY adapters restore the exact saved normal niri frame state,
including nondefault SDR reference white and HDR peak, after success or a returned
error. This is not an arbitrary Smithay override guard, and it does not promise
restoration after panic unwinding. The operation runs between frames, outside
per-content draw overrides. Ordinary SDR captures do not rebuild renderer defaults.
Output PipeWire streams share one prepared scene and color scope once a stream is
ready; inactive or not-yet-due streams do not trigger capture preparation.

The interface is the test surface: real Wayland HDR content is rendered and read
back through GLES and Vulkan across sequential captures, nested SDR scopes and
unsupported framebuffer-format errors. These are renderer checks, not physical HDR
display or complete portal-session acceptance.

## Window unmap

`State::unmap_window` owns closing-animation setup, cast shutdown, MRU removal,
layout removal, transaction deadline registration, focus handling and redraws.
Protocol handlers identify the event rather than repeating this sequence:

- Null-buffer removal starts the closing animation before applying `Window::on_commit`,
  then retains the toplevel as `Unmapped` for a fresh configure/remap handshake.
- Toplevel destruction stops its casts and snapshots the last contents before starting
  the closing animation; it does not retain an unmapped toplevel.

This keeps ordering in one module without widening the layout interface. Protocol
regressions cover MRU/focus after both events and the null-buffer remap handshake;
the existing real-renderer closing-animation test continues to cover snapshot use.

## Repeat the checks

```sh
python3 -B scripts/test-personal-deploy.py
NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 cargo test --locked --workspace
scripts/personal-compare-glass.sh ../Niri-glass ../validation/architecture/glass
```

Deployment tests use temporary roots and mocked authentication/exec boundaries; they
do not exercise real administrator authentication or GDM login. Renderer tests require
an actual EGL/GLES or Vulkan renderer. Software GLES proves shader execution and
pixel behavior, not physical HDR output. Nested desktop checks and real TTY HDR
acceptance remain separate levels of evidence.

The release build must come from a clean revision using `scripts/personal-build.sh`.
When the development worktree has unrelated user edits, use a separate clean
worktree with the pinned adjacent Smithay checkout; preserve the edits in place.
Keep the resulting artifact independent until the user accepts that executable.

## Local evidence, 2026-10-09

Evidence is kept outside the source checkout in `../validation/architecture/`.

- `deployment-tests.log`: 28 tests passed, including source/plan replacement during
  validation/authentication, interrupted bootstrap and the legacy login handoff.
- `workspace-tests.log`: 285 niri library tests, 27 config tests, one config
  integration test, two IPC tests and one doctest passed. The opt-in Glass reference
  comparison was the single ignored test and passed when explicitly run separately.
- `after/corners.log` and `corner-pixel-comparison.json`: 20 before/after PNGs were
  byte-identical, with maximum RGBA difference zero. These cover existing corner,
  Glass mask, fractional border and six soft-shadow cases.
- `after/glass.log` and `glass-pixel-comparison.json`: the six Glass/blur/reference
  appearance PNGs were also byte-identical before and after the refactor.
- Independent read-only review covered deployment loss/race conditions, shader and
  parameter pairing, fallback and HDR scopes, and the GLSL extraction. Both deployment
  findings were fixed and re-reviewed. Test execution is separate implementer evidence.

These measurements use this host's software GLES renderer. They establish equivalence
for the measured scenes and do not assert an unmeasured performance improvement or
new real-display HDR acceptance. Real authentication and GDM login for the new bundle
layout have not been exercised.

The release candidate is `../artifacts/niri-hdr-glass-5617c29e`, built by the normal
locked release helper from clean detached worktree `../architecture-build/niri` at
`5617c29e53cf055bd7e75ce77e33693343812cd1`. Its adjacent Smithay resolves to the clean
pinned checkout. `release-build.log` and `release.json` record the build; SHA-256 is
`226429c29ab6839512b3e77af668d8b7477bf38638b6a60ef04d1f54c17c1fab`.
All five standalone examples, the current canonical config and the prepared migration
candidate validated successfully. The user's uncommitted `daily.kdl.in` output edits
were left in the development worktree and excluded from the release source.

The release also passed an isolated nested Wayland run on `/dev/dri/renderD128`:
`runtime/smooth-nested-5777cd21/` contains nine screenshots, runtime state and logs
covering circular/smooth reloads, resize, asymmetric corners, fullscreen entry/exit,
overview, and restoration. No shader/config errors were logged, and the child exited
normally. This exercises the real GLES backend and screenshots, not physical HDR
output or human acceptance of this new binary.

To repeat that isolated trial from the development checkout:

```sh
python3 scripts/personal-smooth-trial.py \
    --binary ../artifacts/niri-hdr-glass-5617c29e \
    --output-root ../validation/architecture/runtime
```

For a real-console video check, the existing HDR harness accepts the explicit candidate:

```sh
python3 scripts/personal-hdr-video.py --binary ../artifacts/niri-hdr-glass-5617c29e
```

Use its existing TTY procedure and return-to-desktop steps in [VALIDATION.md](VALIDATION.md).
The installed flat daily executable remains the previously accepted SHA-256
`b296f3dce6fa779d114058c4be1ba84a0e23a3ac9ed2062e1c24d6b6ca9bbc3b`.
Neither the candidate nor the new bundle-based deployment has been installed, and no
configuration migration or desktop restart was performed during this refactor.
