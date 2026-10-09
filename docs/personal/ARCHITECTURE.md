# Personal changes: ownership and regression checks

The three refactors start after `a19074380b26899b67e8c9dbcc22aa315a3fdea4`.
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
| Xray placement and drawing | `render_helpers/xray.rs` | Use the prepared background while retaining transforms, clipping, damage and backend dispatch |
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
