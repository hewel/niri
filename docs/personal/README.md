# Personal HDR + Liquid Glass build

This branch adds GLES Liquid Glass to `losnoco/niri:spicy-main`. Spicy remains the direct integration upstream; official niri is a reference, not a second branch to merge wholesale. The system binary, active desktop configuration and services are not replaced.

## Source boundary

The full source records are also machine-readable in [versions.env](versions.env).

| Source | Selected commit |
| --- | --- |
| Spicy niri, immediately below the personal commits | `d9131c336ed291ad235a6b46f9af960a45f2bad4` |
| Spicy Smithay | `a6a4be6d4bc00222e2bc3adbc49dc932b229ff6b` |
| Niri-glass source | `72ee16331994b43a9460b713b828a218a023a857` |
| Official niri base used to derive the Glass changes | `49fc6117fd6c043adaa2ead316b82db5ed735d36` |
| Official main inspected for reference | `2c82b77a7116a1241ea2fd41dd008ff00b609aae` |

Sources: [spicy niri](https://github.com/losnoco/niri), [spicy Smithay](https://github.com/losnoco/smithay), [Niri-glass](https://github.com/zaroutt/Niri-glass), and [official niri](https://github.com/niri-wm/niri).

The selected Glass snapshot's replacements were compared with its pinned official base, including `appearance.rs`, `liquid_glass.rs`, `background_effect.rs`, `framebuffer_effect.rs`, `xray.rs`, both render/shader module files, and `clipped_surface.frag`. Its installer was not run. This port uses a separate `liquid_glass.frag`, retaining spicy's ordinary clipped shader, framebuffer rendering and renderer dispatch. Attribution and source references remain in the adapted code; the repository's GPL license continues to apply.

Local development uses branch `personal/hdr-glass` in `/home/hewel/Codes/niri/personal/niri`. The original `/home/hewel/Codes/niri` checkout remains independent. Tag `personal/spicy-base-20261008` preserves the exact old spicy boundary. To enumerate only the personal patch series:

```sh
git log --reverse --oneline personal/spicy-base-20261008..personal/hdr-glass
```

The series covers configuration/defaults/rule resolution, GLES shaders and integration, SDR/HDR adaptation and fallback behavior, renderer tests, and these examples/build/maintenance documents. It does not contain a bulk official-upstream update.

## Supported combinations

| Combination | Behavior |
| --- | --- |
| GLES, `xray=true`, ordinary SDR sRGB wallpaper/background, SDR output | Glass enabled |
| Same background on an HDR output | Glass enabled before spicy's HDR output conversion |
| `xray=false` | Glass disabled; ordinary blur and other effects retained |
| Vulkan renderer | Glass disabled; existing Vulkan rendering/effects retained |
| HDR, linear or non-sRGB background content | Glass disabled for that background; other effects retained |
| Glass absent or `off` | Existing background behavior; no Glass shader compilation or added Glass pass |

Unsupported effects produce a warning once per unsupported category per compositor process. They do not globally change `xray`. If Glass is the only requested effect and cannot run, no background capture is requested merely for Glass.

Configuration follows `window-rule → background-effect → liquid-glass`:

```kdl
window-rule {
    match app-id="^personal-glass-demo$"
    background-effect {
        xray true
        blur true
        liquid-glass {
            refraction-strength 3
            physical-refraction 1
            saturation 0.9
        }
    }
}
```

Parameters merge across matching rules. A later `liquid-glass { off; }` disables only Glass while preserving inherited parameters. `off false` explicitly re-enables it; an empty later block does not undo an earlier `off`. Invalid values and unknown parameters receive the normal configuration diagnostics. Inactive legacy Glass options are intentionally rejected rather than accepted without an effect.

The color boundary matters: xray backgrounds are captured in SDR even when prepared inside an HDR frame, with the previous blend state restored afterward. Background metadata must identify ordinary SDR sRGB content before the SDR-oriented Glass calculations run. The shader handles premultiplied alpha, adjusts unpremultiplied colors, restores premultiplication, and then enters the existing `niri_blend` output conversion once. Spicy's selected HDR compositing path stores PQ/BT.2020 encoded values; it is not a wholly linear FP16 pipeline, and HDR values need not exceed one. Framebuffer-derived backgrounds keep spicy's existing compositing-space blend parameters and do not run Glass. Direct scanout remains governed by the base compositor's existing eligibility checks; a visible sampling effect uses compositing.

## Reproduce and build

The sibling layout is required by **both** Cargo overrides:

```text
integration/
  niri/       # personal/hdr-glass
  smithay/    # the exact SMITHAY_REV
    smithay-drm-extras/
```

`Cargo.lock` pins crate dependencies but cannot pin the Git revision of a path checkout. Always retain the separate Smithay record. No personal remote was created or published. A clean local clone of the completed branch can be prepared from the existing repository:

```sh
mkdir -p /tmp/niri-personal-reproduction
git clone --no-hardlinks --branch personal/hdr-glass \
    /home/hewel/Codes/niri /tmp/niri-personal-reproduction/niri
git clone https://github.com/losnoco/smithay.git \
    /tmp/niri-personal-reproduction/smithay
cd /tmp/niri-personal-reproduction/niri
source docs/personal/versions.env
git -C ../smithay checkout --detach "$SMITHAY_REV"
./scripts/personal-build.sh
```

For another machine, first transfer this repository/branch or a Git bundle preserving the personal commits and base tag. Substitute that source for the local clone path. Cloning only spicy does not include the personal patches.

The selected niri and Smithay manifests declare Rust 1.87 as their minimum. The locked dependency set was tested with Rust/Cargo 1.98.0 on CachyOS; the minimum toolchain was not tested. Upstream Cargo features are unchanged, including Vulkan support. The build requires a C toolchain, `pkg-config`, Wayland, xkbcommon, libinput, libudev, libseat, EGL/GLES/GBM, libdisplay-info, Pango/Cairo, PipeWire, and Vulkan headers/loader. On this Arch-derived host, libseat is supplied by `seatd`. Check the repository's normal build documentation for package equivalents on other distributions.

The helper verifies the exact adjacent Smithay revision, rejects tracked modifications in either checkout, checks the recorded spicy ancestry, builds a release executable with `--locked`, and validates both example configurations. It records niri's full HEAD through the existing `NIRI_BUILD_COMMIT` version mechanism. It does not fetch changing branch tips, install binaries or edit session configuration. Commit intended changes before running this reproducible build helper.

The helper explicitly builds into this checkout's `target/`, including when a caller has set `CARGO_TARGET_DIR` or a Cargo target-dir configuration. Its release binary is `target/release/niri`. The independent trial artifact on this host is `/home/hewel/Codes/niri/personal/artifacts/niri-hdr-glass`. The unmodified debug baseline is retained as `niri-spicy-baseline` beside it.

## Independent trial

From the current graphical session, launch a separate nested window:

```sh
/home/hewel/Codes/niri/personal/artifacts/niri-hdr-glass \
    -c /home/hewel/Codes/niri/personal/niri/examples/personal/glass.kdl \
    -- python3 /home/hewel/Codes/niri/personal/niri/examples/personal/demo.py
```

The demo supplies a static SDR layer-shell background and GTK window made translucent by the example window rule. **swaybg and foot are not required for the demo.** It uses the already installed GTK 3, Python GObject/Cairo and GtkLayerShell; these are not new compositor build dependencies. The optional `Mod+Return` binding starts foot; foot 1.28 uses `colors-dark.alpha` and `colors-dark.blur` in the supplied overrides.

The base compositor's client blur-region policy is preserved: a client explicitly setting an empty region suppresses the background effect. A configured Glass rule therefore does not guarantee that every client will display Glass. The supplied GTK window demonstrates the compositor's configured effect without depending on foot's region requests.

The example sets `draw-border-with-background false`: spicy's default filled focus-ring background for CSD windows would otherwise hide the sampled wallpaper under the focused window. This is a sample configuration choice, not a change to the compositor's default policy.

Do not add `--session` for this independent trial. `Mod+O` toggles overview, `Mod+F` fullscreen and `Mod+Shift+E` exits only the trial compositor. Niri's default `Mod` is Alt in a nested session and Super on the TTY backend. The examples have no desktop autostarts and the SDR example does not enable HDR. Keep experimental changes in copies of these files.

For a real TTY/DRM HDR trial and expected observations, follow [VALIDATION.md](VALIDATION.md). Nested rendering cannot establish physical HDR correctness.

## Update safely

1. Keep the old `versions.env`, base tag, Smithay revision and explicit personal commit list before fetching. Update a separate integration branch/worktree and build/test the proposed niri + Smithay pair first. Matching branch names alone are not compatibility evidence.
2. Follow `losnoco/niri:spicy-main`. When the saved old base is still an ancestor of the new spicy base and the commits after it are only the intended personal series, normal rebasing onto the fetched spicy branch is appropriate. Re-run the baseline checks on the newly selected dependency pair.
3. If spicy rewrites history, transplant only the personal patches. For a clean linear series with the explicit saved boundary, use `git rebase --onto <new-spicy-base> <saved-old-spicy-base> <new-integration-branch>`. Do not substitute a guessed current merge-base for the saved old boundary.
4. For complex history, start a new branch at the selected new spicy base and cherry-pick the explicitly identified personal commits in order. Resolve each port against the new color pipeline and renderer interfaces, then validate it again.
5. Record new full SHAs and a new immutable base tag only after selecting the combination deliberately. Preserve the old records. Do not automatically force-push.

Monitor official niri for relevant fixes. Bring an official fix in early only for a specific need, after checking its dependencies and whether spicy already implements it. Identify such a commit as a temporary downstream patch, and skip/remove it when spicy includes an equivalent fix. No unattended fetch/merge/install updater is included.
