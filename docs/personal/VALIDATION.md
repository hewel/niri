# Validation record and display checklist

## Window-history final ACK acceptance, 2026-10-23

The coordinator has closed the scoped window-privacy acceptance. Independent
review `/tmp/niri-native-config-ack-independent-review-20261023/REPORT.md`
reports Standards0/Spec0 and closes the P2 application-order evidence gap.
This is acceptance of the local candidate, not installation or deployment.
The executable remains
`3acf7953e308b3ed86971232ae8244165606ad353c85108a36cf01d668a0af50`;
the release rlib remains
`6eda8283613de14b95bfb0f12632078848f43d6fbc98d7bf288e54062c608096`.

The original four-case run below used a watcher parsing log, which did not prove
that minimized tightening had been applied before restore. That P2 and the
44 original PNGs remain unchanged; the old barrier is not retrospectively valid.
The authorized m-cycle-only supplement is
`/tmp/niri-gpu-capture-pfaikeuf/REPORT.md`, with raw fixture data under
`/tmp/niri-fixed-privacy-7vhritr0/m-cycle/`. It ran
04:05:16.644895–04:05:33.029114 +0800 (16.384 seconds), on private
1920×1080 scale1 Gamescope/RADV → Niri host → native candidate,
with RX 7900 XTX / radeonsi GLES rendering.

The subscription discarded initial ConfigLoaded replay seq6. Fresh successful
application ACKs seq17 and seq18 both preceded restore seq20, with the same
window minimized, parent-commit-count4 and capture-count5. No capture occurred
between the rule transitions. Immutable private configurations, one outstanding
load and rejection of unexpected notifications establish this fixture's
correlation; ConfigLoaded itself has no request ID. Receipt timestamps of later
drained events are not treated as compositor emission timestamps.

Independent recomputation passed all13 new PNGs. After restore and after pending
close, both capture routes had zero old red/green pixels; private-parent Output
retained each old color (79680 and77836 pixels respectively). Before close,
legitimate live gray bars remained visible with delta144 and no old colors.
The previous44PNG and ordinary resize/pending controls were reused through
unchanged dependencies, not rerun. Software minimized/serial independent review
remains `/tmp/niri-minimized-policy-corrected-review-20261023/REPORT.md`.

This supplement's grim transport is directly evidenced as ExtImageCopyCapture,
using ScreenCapture, as does IPC screenshot. Parent capture observes the
candidate's presented Output; it is not direct Output readback. There is no
hardware Screencast, portal/PipeWire, physical-display, HDR/color-quality,
Vulkan-candidate, device-fault or performance acceptance.

All57 known owned PIDs were absent; STOP was restored and the authorized release
notification sent. Three exit statuses remain unknown. Xwayland and the quit IPC
helper exited1; candidate, client, host and Gamescope exited0. These distinctions
are preserved in `native-slot-release.json` and the independent cleanup audit.
No surface, build or passing check was rerun for this final documentation update.
Earlier failures, P2 evidence, seven existing clippy warnings and unrelated
workspace-formatting failures retain their recorded outcomes.

## Window-history four-case native evidence on 3acf, 2026-10-23

Historical checkpoint: native execution passed the four fixture scenarios.
The later P2 application-order gap and its closure are recorded above; this
checkpoint alone does not prove minimized application-before-restore.
Executable SHA-256:
`3acf7953e308b3ed86971232ae8244165606ad353c85108a36cf01d668a0af50`;
release rlib SHA-256:
`6eda8283613de14b95bfb0f12632078848f43d6fbc98d7bf288e54062c608096`.
The fixed runner and all five fixture assets matched the authorized manifest
before and after execution. No product source changed for the native fixtures.

Independent minimized re-review is recorded at
`/tmp/niri-minimized-policy-corrected-review-20261023/REPORT.md`: Standards0/Spec0,
three minimized after controls plus a real ack/commit/restore serial control on
software GLES. Its initial incorrect minimized-Output oracle remains preserved.
That review is separate from, and not a substitute for, the native run below.

The accepted private Gamescope→Xwayland Niri host→native Wayland candidate run is
`/tmp/niri-gpu-capture-5hg5j42d/`; original case data and images are under
`/tmp/niri-fixed-privacy-bh4ba7jf/`. It ran from
`03:46:36.622602` to `03:47:13.151352 +0800`, 36.529 seconds including cleanup.
Gamescope selected RX7900XTX/RADV; host and all four candidates used
RX7900XTX/radeonsi GLES3.2 on `/dev/dri/renderD128`. Host and candidates were
1920×1080 scale1. Initial candidate and private-parent images passed exact 1:1
background/canary geometry checks. Host/candidate overview observations were
closed, including all 21 parent-capture checks.

All four scoped scenarios passed. Counts below are red/green historical-canary
pixels in the fixed window ROI; closing captures occur at different times.

| Scenario and measured phase | Candidate IPC | Candidate grim/scopy | Private-parent Output |
| --- | ---: | ---: | ---: |
| Active resize, after tightening | 0/0 | 0/0 | 79040/79040 |
| Pending positive control, after closing | 78400/78400 | 78155/78155 | 77836/77836 |
| Pending tighten→loosen without capture, after closing | 0/0 | 0/0 | 77836/77836 |
| Same cycle while minimized, restore→close | 0/0 | 0/0 | 77836/77836 |

All three pending scenarios first completed the live resize into neutral gray
bars: old canary counts zero, top/bottom blue-channel delta144, in both allowed
live IPC and private Output. The later return of old colors in closing Output,
and in the unprotected pending control, proves actual historical pending
consumption rather than an empty fixture. Pending was created 0.310–0.354 seconds
into the configured 8-second resize. Wire logs prove child commit, parent commit,
two completed syncs, then child destroy; there was no further parent commit until
null-buffer close. Both policy cycles kept capture-count5 and parent-commit-count4
through tightening and loosening. Minimized IPC states retained the same window
through both changes and restored it before closing.

All 44 original PNGs were hashed and their canary/gray/black measurements
recomputed. Every capture operation took under 0.114 seconds; the latest closing
capture was under 0.255 seconds after close, within the unchanged 3-second limits.
See `pixel-audit.json`, `protocol-audit.json`, `artifacts.json` and `REPORT.md`.
Candidate IPC and grim/scopy both exercise `RenderTarget::ScreenCapture`
(`niri.rs:7138`, `6391`, `6475`). Private-parent grim observes the candidate's
presented Output buffer; it is not a direct candidate Output capture request.
This round does not exercise the PipeWire `Screencast` target.

All 152 known owned PIDs were absent after cleanup. Host, four candidates and
four synthetic clients exited0. Three wrapper/reaper exit statuses remain
unknown; absence is not reported as exit0. `native-slot-release.json` records
active=false and the single coordinator notification. STOP was restored at that
checkpoint; the later explicitly authorized ACK supplement is recorded above.
Shared Gamescope439607 was untouched.

The earlier 10.926-second attempt and all32 failed-prerequisite PNGs remain at
`/tmp/niri-gpu-capture-w6ubsky3/` and `/tmp/niri-fixed-privacy-ohtboqoh/`.
It showed a centered half-size parent presentation before any policy transition.
Its overview explanation remains an inference: this successful run recorded
closed overview states, not the earlier run's missing state. The fixture was
corrected by explicit private overview control and stricter geometry checks,
not by weakening privacy assertions or changing product code.

No physical-display, portal/PipeWire, HDR, Vulkan-candidate or performance
acceptance is implied. Older 6d hardware records retain their original version;
they were not substituted for this 3acf window-history run. Final release and
deployment decisions remain with the coordinator.

## Minimized window policy propagation, 2026-10-23

Independent review of the window-history increment passed four ordinary/title
after scenarios but found a missing minimized-pool update route. On executable
`2bd49b424ccc2e6ce832adfb78ef2771ada7d7d9f2de4d5e340560b6899c98ac` / rlib
`5f32f456ea7a4fab23008a1900fd3d752bd0a14cdcf91f710ef78cab21cbf996`, both
minimize→tighten and minimized tighten→loosen restored old pending capture pixels;
tighten-before-minimize was safe. The confirmed report and before evidence remain
unchanged at `/tmp/niri-window-policy-independent-after-20261023/REPORT.md`.
These failures were not rerun, and earlier background/target/remap acceptance is
not reopened.

`Layout::update_window` now forwards minimized tiles through the same commit-hook
then tile-update ordering used for interactive moves. Resolved rules restrict old
samples immediately while hidden, including a tightening that is later removed
before any capture. The existing `RenderSnapshot` restriction is reused; no new
policy record, restore-time repair, workspace movement or output binding is added.

The new two-color regression covers the three ordering controls for pending,
active resize and pending baked during resize. It checks retained mapped identity,
live visibility after restore, closing privacy and Output preservation without
parent commits between rule transitions.

Artifacts and the pre-correction source/release baseline are under
`/tmp/niri-window-policy-fix-20261023/minimized-fix/`. Candidate executable SHA-256:
`3acf7953e308b3ed86971232ae8244165606ad353c85108a36cf01d668a0af50`;
release rlib SHA-256:
`6eda8283613de14b95bfb0f12632078848f43d6fbc98d7bf288e54062c608096`.

Focused verification passed: 10 capture-privacy tests (the new test exercises nine
ordering/sample combinations), all six existing minimize/restore lifecycle tests,
no-default-features check, release build, and all-target clippy with the same seven
existing warnings. This is not strict warning-free clippy. Changed-file formatting
passed; the previously recorded unrelated whole-workspace formatting differences
were not reformatted or rechecked. No unrelated Python/full-suite rerun.
The initial new regression did not compile because one redundant assertion mixed
client/server surface types; that assertion was removed, while retained mapped
identity and actual pixels remain checked. The initial log is preserved.

The three independent minimized fixtures were linked against this new release
rlib and passed on actual `llvmpipe (LLVM 23.1.1, 256 bits)` GLES, not hardware.
All 36 RGBA files were independently byte-counted against the probe output.
Both formerly failing closing cases now have red/green counts `(0, 0)` for
ScreenCapture and Screencast, while Output keeps `(2047, 2048)`.
The no-capture tighten→loosen case permits current live `(2048, 2048)` before
closing, but does not resurrect pending samples. Tighten-before-minimize remains
safe. The three probes ran from `03:16:37.901422` to `03:16:39.157383 +0800`;
PIDs `630994`, `631011`, `631024` each exited zero and were absent after wait.
See `results.json`, `pixel-audit.json`, `cleanup.json` and `gates-after.json`.

At this software-only checkpoint, native work was held: the planned old-version
wrapper never started, so there were no new owned display processes to clean up.
`/tmp/niri-window-policy-fix-20261023/native-slot-release.json` records that state
and the coordinator received the single authorized slot-release notification.
Independent re-review and the separately authorized 3acf hardware/ACK evidence
have since completed, as recorded above. Earlier 6d hardware results were not
substituted for this candidate. No physical-display, portal/PipeWire or HDR
acceptance is added.

## Dynamic window historical samples, 2026-10-23

This is a separate correction after the completed background/target/remap acceptance
below; it does not reopen those findings or reclassify their historical results.
The independent before evidence is preserved unchanged at
`/tmp/niri-window-policy-boundary-20261023/REPORT.md`: on release executable
`6d590f76eab9d6e219f48a06bacf71e60d1181fbc0093f8c7a0ead0309a41416` and rlib
`0aaec3b7084f41b8e1806fbedb2a27e2774db336aba5b9498deb895f6265d502`, active resize
retained old window pixels after resolved ScreenCapture tightening, and consuming
a previously saved pending snapshot restored old pixels after a blocked live frame.
The six software probe groups had two failures and four passing controls; they were
not rerun to confirm the failure.

`Tile::update_window` now monotonically restricts existing resize and pending samples
as rules change, including tighten-then-loosen with no capture in between. The
existing snapshot protection/target-slot selection provides a black resize
replacement or an unavailable pending target. Prebaked captures cannot bypass it.
Protected resize texture failures use a black capture fallback; Output retains
its original animation and error behavior. Loosening may expose current live
content, not revive a previously restricted old sample. No global window history
or post-destruction rule tracking is introduced.

Regression coverage uses two-color real window textures and frozen animation time.
It retains the earlier Screencast target-sharing control, while intentionally
changing its ScreenCapture expectation to the new dynamic-window contract.
New cases cover live resize tightening/pre-protection/loosening, no-capture rule
transitions, pending consumption/removal, no-pending controls and Output retention.
Pending created during active resize also exercises an already baked ScreenCapture
slot, so merely changing the snapshot's policy field cannot satisfy the regression.

Verification and incremental handoff: `/tmp/niri-window-policy-fix-20261023/REPORT.md`,
with original before artifacts, `incremental.patch` and `artifacts.json`.

- All nine distinct privacy tests passed, plus four layer-shell and two window-unmap
  tests. The first privacy invocation passed eight and exposed a test-clock mistake
  in the new midpoint control: time adjustment is lazy and must be read before
  freezing the rate again. The corrected test passed separately; its clock now
  asserts the actual 4000ms advance. The extended baked-pending test also passed.
  Passing unrelated tests were reused, not repeatedly run.
- No-default check and release build passed. All-target clippy and the final
  test-only follow-up exited0 with the seven existing warnings; not strict lint green.
  Changed Rust files were formatted. Whole-workspace `fmt --check` failed on existing
  differences in ten untouched files, including backend TTY, workspace, shader and
  other test/UI files. Those unrelated files were not reformatted; its full output
  remains in `format-workspace-check.log`.
- The newly built release executable SHA-256 is
  `2bd49b424ccc2e6ce832adfb78ef2771ada7d7d9f2de4d5e340560b6899c98ac`;
  release rlib SHA-256 is
  `5f32f456ea7a4fab23008a1900fd3d752bd0a14cdcf91f710ef78cab21cbf996`.
  The original six independent software probe scenarios, plus resize and pending
  tighten-then-loosen cycles without intermediate capture, passed against this rlib.
  The actual renderer was llvmpipe; all 84 RGBA readbacks matched their logged
  red/green counts. All eight owned probe processes exited0 and were absent afterward.
- Resize tightening now gives red0/green0 at both 0ms and 4000ms, while Output
  retains its two-color animation. Pending tightening gives red0/green0 after close.
  A no-capture tighten/loosen cycle leaves the old resize input black and pending
  capture absent; currently allowed live content returns as the resize advances or
  before consuming pending. Pre-protected, loosening and no-pending controls passed.

At this historical 2bd checkpoint, hardware/native capture acceptance and
independent approval were still outstanding. The minimized omission found later
required the 3acf correction and its separately recorded acceptance above.
This stage used software headless/CPU only, with no native display surface or
real desktop/session/portal operation. Prior hardware evidence belonged to an
older executable. No unrelated Python or full-workspace suite, installation,
commit, push or deployment was performed.

## Dynamic background capture protection, 2026-10-23

The capture/background snapshot fix records source-layer protection generations
through pending unmap and closing animation lifetimes. A tightened target omits
old closing contents instead of reusing an unprotected texture; Output retains its
animation. Snapshot baking distinguishes ScreenCapture from Screencast and shares
equivalent variants. [ARCHITECTURE.md](ARCHITECTURE.md) documents the invariants and
resource tradeoff. This remains an uncommitted development candidate, not a daily
selection. Scoped isolated-GPU runtime acceptance is complete as recorded below.

Independent review subsequently found a P1 in the original variant-sharing key:
an active resize retains its old window protection policy, so equal current rules
did not prove equal ScreenCapture/Screencast pixels. Before-fix evidence and the
unchanged-rule control are preserved at
`/tmp/niri-capture-independent-review-20261023/REPORT.md`. The correction disables
cross-target sharing while an older resize snapshot contributes, without dropping
that snapshot or its protection checks; a deterministic pixel regression includes
the unchanged-rule control. Affected checks, independent code/headless re-review,
and scoped nested capture now pass. No deployment approval is implied.

A second independent P1 showed that rebuilding `MappedLayer` on same-object remap
severed old snapshots from later policy updates. The original evidence remains at
`/tmp/niri-layer-remap-review-20261023/REPORT.md`. The correction keeps the policy
record and last resolved protection on the original layer-shell `LayerSurface`,
updates unmapped objects too, and does not revoke snapshots merely on unmap.
Regressions cover closing/pending remap with continuous-mapped controls and changes
while unmapped through reload, startup and commit. The affected checks below ran
after both corrections; earlier broad results remain separately labeled historical.

Observed checks after both P1 corrections:

- All seven capture-privacy tests passed with required real headless GLES rendering
  (`NIRI_TEST_REQUIRE_GPU=1`, explicitly software llvmpipe). Resize screenshots retain
  4096 old allowed pixels while Screencast remains red0 before/after close, including
  the unchanged-rule control. Layer remap/unmapped changes retain old Output canary
  pixels while both capture targets remain red0. No before-fix reproduction rerun.
- Four existing layer-shell and two window-unmap tests passed: 13 affected tests,
  zero failures across these three focused invocations.
- Locked no-default-features check and release build passed. Package/all-target
  clippy exited zero with the same seven pre-existing warnings, not strict lint green.
  The six changed Rust files were formatted with nightly rustfmt.
- Logs and exact commands: `/tmp/niri-capture-policy-fix-20261023/review-gates.json`
  and `review-*.log`. Corrected release SHA-256:
  `6d590f76eab9d6e219f48a06bacf71e60d1181fbc0093f8c7a0ead0309a41416`.
- Independent re-review reports Standards 0 / Spec 0, closes both P1 findings, and
  passes 12/12 release-rlib software GLES after controls:
  `/tmp/niri-capture-corrected-review-20261023/REPORT.md`. Its reviewed patch SHA-256
  is `687edd3ddcf03b3f9562d7af9ac8c8b5ccb72e64ac3fd2894f1cc8717683725b`;
  the binary matches the hash above. Source/test/architecture files are unchanged
  since that review; later changes here only record the GPU smoke outcome.
  These independent probes were reused, not rerun, and do not replace nested smoke.

Historical non-display checks before the two review corrections:

- The five new `tests::capture_privacy` regressions passed with
  `NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 cargo test --locked --package niri
  --lib tests::capture_privacy -- --nocapture`. They render real headless GLES scene
  pixels for Output, ScreenCapture and Screencast. They cover protection tightening
  during closing and before consumption of a pending subsurface-destruction snapshot,
  rule removal without snapshot revival, mixed capture policies, and commit-driven
  layer matching. The pending case changes the wallpaper before final destruction,
  proving Output retains the earlier snapshot rather than rebuilding it.
- The workspace run passed the other 293 Niri tests, with the existing optional Glass
  comparison ignored. Its five failures were the new fixture's initial layer-shell
  sizing; a focused rerun then exposed its draw-order mistake. Both fixture issues
  were corrected without changing expected privacy outcomes, and the five tests
  passed. The already-passing 293 tests were not rerun.
- The remaining workspace packages passed 27 config tests, one wiki integration
  test, two IPC tests and one IPC doctest. Combined exercised result: 329 passing
  tests and one existing ignored test, across separate runs rather than a claimed
  single all-green invocation.
- Locked release build, no-default-features check and workspace/all-target clippy
  completed. Clippy warnings are not a strict warnings-as-errors pass.

Evidence and display-validation status:

- Before-fix dynamic pixel evidence is preserved unchanged at
  `/tmp/niri-dynamic-privacy-fwrn1vv4/`: P2 exposed red pixels only inside the old
  closing window after its still-live background layer became protected; P3
  over-redacted a screencast-only background in ordinary screenshots.
- Current logs, focused-test result, isolated smoke drivers and review scope are
  under `/tmp/niri-capture-policy-fix-20261023/`.
- The first after-fix nested attempt, `/tmp/niri-fixed-privacy-3vsecyrv/`, timed out
  before its synthetic client became ready and produced no captures. Its own
  SIGTERM teardown also timed out; only that fixture's exact verified process was
  force-cleaned. No subsequent dynamic or static scenario started. The cause is
  unresolved; this does not establish a privacy failure or a successful smoke.
- The second serialized real-parent attempt also timed out; its exact owned
  processes were cleaned. The coordinator subsequently confirmed the real session
  was locked. No further locked-parent trial is allowed.
- A software Xvfb→Niri isolated attempt reached protected/closing pixels with red
  count zero, but failed the unchanged `black > 1000000` oracle: 1272×768 images
  contain only 976896 pixels. This is not a full smoke pass. The assertion and
  deadline are retained; the prepared replacement environment must establish an
  actual 1920×1080 candidate output before running the same fixture.
- One authorized independent GPU wrapper ran from 01:45:10.569 to 01:45:25.552
  +0800 (14.982 seconds), evidence `/tmp/niri-gpu-capture-f1qz00na/`. Gamescope
  selected RX 7900 XTX RADV; host and native-Wayland candidate initialized radeonsi
  GLES 3.2. The host reached 1920×1080, scale 1.
- This attempt failed an environment prerequisite before any capture or policy
  transition: the nested fixture directory made the candidate IPC pathname 115
  bytes, exceeding Linux's 107-byte pathname limit. The log reports
  `path must be shorter than SUN_LEN`; readiness expired at its unchanged 8-second
  deadline. Candidate output dimensions and first-image pixels were not obtained.
- All recorded owned PIDs were absent after cleanup; known statuses and uncollected
  descendant statuses are explicitly distinguished in `failure-and-exits.json`.
  Shared Gamescope and the retained Xvfb fallback were untouched. No unapproved retry.
  The next preparation restores the shallow fixture path, adds a pre-launch path
  length check, and successfully binds a 93-byte disposable local Unix socket.
  That check creates no display and is not a completed capture smoke.
- After separate authorization, the shallow-path GPU wrapper ran at
  01:56:39.016–01:56:51.366 +0800 (12.350 seconds), evidence
  `/tmp/niri-gpu-capture-8nx8z5x9/REPORT.md`. Gamescope used RX7900XTX RADV;
  host and four candidates used radeonsi GLES3.2. Host/candidate IPC all reported
  1920×1080, scale1; all 19 recorded images have that size. Binary unchanged.
- Three scenarios passed their original IPC/grim assertions: late layer protection
  gave closing red0/black2073600, removal kept red0; screencast-only layer and window
  backgrounds retained allowed red pixels in ordinary screenshots. All closing
  captures remained inside their original three-second bound.
- Pending positive control failed: expected old red >5000, observed red0,
  white18024/blue2053026. Execution stopped before its grim capture and before
  pending-protection. The temporary helper committed a synchronized child without
  its parent, so its roundtrip did not establish the applied child→root state
  required to create a pending snapshot. A one-parent-commit fixture correction
  compiled; runtime verification followed in the final authorized attempt below.
  No product change or oracle relaxation.
- All 90 recorded owned PIDs were absent after cleanup; six descendant exit codes
  remain explicitly unknown. GPU overlap with the separately isolated JellyPilot
  functional run was permitted, so no performance inference is made. Both attempts
  and first-round unknown exit statuses remain preserved. At that point full
  nested acceptance remained incomplete.
- The final authorized wrapper ran at 02:08:21.985454–02:08:37.177900 +0800
  (15.192609 seconds), evidence `/tmp/niri-gpu-capture-wgq3gdk1/REPORT.md`.
  It ran only pending-control/pending-protection and the four static cases;
  the preceding three successful dynamic cases and CPU gates were not repeated.
  The executable remained SHA-256
  `6d590f76eab9d6e219f48a06bacf71e60d1181fbc0093f8c7a0ead0309a41416`.
- Gamescope used RX7900XTX RADV. Host and all six candidates used radeonsi,
  GLES3.2 Mesa26.2.4-arch3.1, `/dev/dri/renderD128`; output IPC confirmed
  1920×1080, scale1. All 25 images had that geometry and independently reread
  red counts matched the fixture records.
- The corrected pending positive control retained red133374/133852 in IPC/grim.
  Pending protection gave red0/black2073600 in both; removing protection kept red0.
  The latest pending post-close capture was 0.243391 seconds, inside the original
  three-second limit.
- All four static background/window × protected/control cases passed mapped and
  closing IPC/grim checks. Protected background retained its white marker with
  red0; protected window gave red0/white0. Both closing controls retained red
  >130000, white markers and the changed blue background. The latest capture was
  0.311809 seconds, inside the original 1.5-second limit. No assertions or
  readiness/capture bounds were weakened.
- All 107 recorded owned PIDs were absent after child→host→Gamescope cleanup.
  Supervisor, Gamescope, host and six candidates exited0; private Xwayland exited1
  during teardown. Eight reaper/launcher/GTK exit codes were not collected and
  remain unknown in `pixels-and-exits.json`; earlier unknown statuses are unchanged.
  No owned surface remains. Shared Gamescope439607 was untouched. The coordinator
  separately retired the unused software Xvfb fallback using its STOP sentinel;
  it is no longer retained as a running service.

The combined acceptance matrix is five dynamic and four static real-GPU scenarios
on the unchanged release executable, plus the independently reviewed 12/12
software-GLES P1 resize/remap/unmapped-policy controls on release rlib SHA-256
`0aaec3b7084f41b8e1806fbedb2a27e2774db336aba5b9498deb895f6265d502`.
The final GPU wrapper did not duplicate that P1 matrix. Independent review remains
Standards0/Spec0; subsequent repository changes only record validation results.
The consolidated report links each version, fixture, original failure and cleanup
ledger; this is not a single all-green full-suite run or a performance claim.

No real desktop capture, portal authorization, configuration/service change,
installation/rollback, daily selection, restart, commit or push was performed.
The existing five-file deployment/migration fix and user `validation/` are preserved;
its 52 Python tests were not rerun. Actual PipeWire delivery and physical HDR/TTY
are not exercised here. IPC/grim hardware evidence does not imply portal/PipeWire
Screencast delivery, human color-quality or physical-display acceptance.

## Migration displaced-file preservation, 2026-10-22

Forward application, interrupted recovery and failed-validation rollback now share
an intent-before-exchange protocol. Linux `renameat2(RENAME_EXCHANGE)` retains the
actual displaced inode under the stage's private `recovery/replace-*/saved` path,
without a missing live-path interval. The accompanying intent records the target,
replacement inode and allowed hashes. Retry distinguishes an unexchanged replacement
from a displaced file, while old prepared/applying manifests remain supported.
Unknown retained or live contents stop migration and report the evidence paths.
Recovery files are never automatically deleted, including after successful migration,
because an editor may still hold an old inode open. The migration lock serializes
only migration writers and is not an editor-cooperation assumption.

Completed verification after code and regression edits:

- `python3 -B scripts/test-personal-deploy.py`: 52 passed. Twelve additional
  deterministic cases cover cross-file saves during recovery and rollback, in-place
  and atomic saves immediately before exchange, an atomic save immediately after
  exchange, late writes through an old open descriptor, process exits before intent
  publication/before exchange/after exchange/during recovery, a displaced user save
  followed immediately by process exit, unsupported exchange, and refusal by the
  real session shell before service startup. Existing deployment and migration
  cases passed in the same gate; no sleep-based races were used.
- A throwaway `/tmp` smoke used the real migration CLI, session shell and
  `niri 26.04 (ad20c3181084e36b8a89a47ace340c0528ae7460)` validator. It recovered
  consecutive application/recovery process exits and preserved user saves during
  recovery and rollback. The rollback case caused an actual KDL parse failure in
  the isolated live config, rather than replacing the validator with a success stub.
  An atomic editor save at exchange was retained and prevented session startup.
  A save through an old descriptor after successful application remained in recovery;
  the next real CLI apply refused to continue. All smoke assertions passed.
  Temporary smoke fixtures and the driver were removed afterward.

No real config, installation, rollback, privilege escalation, user-manager operation,
desktop lifecycle, commit or push was performed. The session probes used a
fixture-only `systemctl` logger; no service was started. The prior four-file deployment
fix and the user's `validation/` were preserved. No Rust surface changed.

Limits: this verifies process-crash recovery, not power-loss durability or actual
display-manager login. Exchange requires supporting Linux/filesystem behavior and
a same-filesystem stage; unsupported exchange has no destructive fallback. A save
that occurs only after migration returns is retained through the old inode and
detected at the next check; this does not predict future writes or stop an already
started session. This protocol still requires independent data-safety review.

## Daily deployment safety fixes, 2026-10-22

The three isolated deployment findings are addressed without changing renderer code:

- Published store, releases and bundle directories, and newly created required
  parents, receive explicit `0755` modes. Staging stays private; existing general
  system-directory permissions are preserved. Dedicated deployment-directory
  symlinks are rejected instead of relaxing another directory's permissions.
- Rollback without system entry changes does not reload user-unit metadata.
  Installation reload failure after publication reports the committed selection
  and rollback target, with a safe same-artifact/same-acceptance retry instruction.
- Acceptance-only changes preserve the effective rollback target by comparing
  verified `niri`, `session.sh` and `unify-config.py` contents. First installation
  and equivalent legacy imports do not invent duplicate rollback targets.
  Full bundle integrity and expected-selection checks remain in force.

Completed verification:

- `python3 -B scripts/test-personal-deploy.py`: 40 passed, including ten new
  regressions for permissions, private staging, symlink refusal, reload failure
  and retry, acceptance transitions, legacy bootstrap and helper changes.
  The first run exposed an overly narrow test process allowlist, not a product
  failure; the authorization fixture now creates its staging under its isolated
  root, and the complete gate passed after that correction.
- A throwaway `/tmp` smoke exercised the real `Deployment`, publisher and Niri
  validators with the existing `1a230591` and `ad20c318` release binaries. With
  umask `077`, published directories were `0755`. A metadata-only update retained
  the distinct older rollback build. A simulated missing user manager produced
  the explicit already-selected error; retry preserved rollback, and rollback
  succeeded while the user manager remained unavailable. Selected and rolled-back
  binaries reported the expected versions. Fixture config bytes were unchanged.
  The temporary root and smoke driver were removed after verification.

All publication and selection changes in these checks were confined to temporary
filesystems. Privilege escalation and user-manager commands were intercepted;
no real `sudo`, `systemctl`, installation, rollback, desktop restart, configuration
change, commit or push was performed. The user's untracked `validation/` directory
was not changed. Previously recorded Rust checks are not rerun for these Python-only changes.
Real privileged ownership, display-manager login and power-loss durability are
not established by this fixture verification. Independent review remains a
separate coordinator step.

## Xray, SDR capture and window unmap, 2026-10-22

The three architecture candidates are implemented on top of
`1a2305913fb59cafab15ee7d689627c05ce8a7f8`; their ownership contracts are in
[ARCHITECTURE.md](ARCHITECTURE.md). The adjacent pinned Smithay and the user's
untracked `validation/` directory were not changed.

Completed checks:

- `cargo build --release --locked --package niri`: passed. The local candidate is
  `target/release/niri`, reporting `niri 26.04 (v26.04-297-g1a230591-modified)`.
  This is a development-worktree candidate, not a published or accepted daily build.
- `NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 cargo test --locked --workspace
  --exclude niri-visual-tests`: 324 passed, one existing opt-in Glass comparison
  ignored. New tests cover real GLES/Vulkan HDR-content capture and restoration,
  Xray target routing and frame cleanup, and Wayland unmap/destruction/remap behavior.
  The existing real-renderer closing-animation test also passed.
- Locked feature checks passed with no default features, individually with `dbus`,
  `systemd`, `dinit` and `xdp-gnome-screencast`, with default features, and with
  `profile-with-tracy`.
- `cargo clippy --locked --workspace --all-targets`: passed with warnings.
  No unrelated lint fixes were applied. Workspace formatting still reports existing
  formatting debt; changed capture scopes and new tests were formatted without
  reformatting unrelated code.
- Two independent read-only reviews found no actionable patch-introduced issues
  in capture state/restoration or Xray privacy/lifetime and window-exit ordering.
  These reviews did not independently execute tests or a compositor.

An isolated nested release compositor exercised circular/smooth corners, resize,
asymmetric corners, fullscreen entry/exit, overview and restoration. All nine
1871×2135 screenshots matched the baseline's decoded RGBA bytes exactly, with
maximum channel difference zero. Evidence:

- Before: `/tmp/niri-architecture-20261022-before/smooth-nested-q07q9oui/`
- After: `/tmp/niri-architecture-20261022-after/smooth-nested-_devfpur/`
- Comparison: `/tmp/niri-architecture-20261022-after/pixel-comparison.json`

A second isolated run exercised screen and window screenshots, external `grim`
capture, GTK window destruction followed by a new mapped window, and the screenshot
selection interface captured by `grim`. Its screenshots and `result.json` are in
`/tmp/niri-architecture-capture-gdj1ax4k/`. Both child compositors exited with status
0; neither log contained warnings or errors. The screenshot selector and ordinary
Glass scene were visually inspected. Temporary evidence under `/tmp` is not durable.

No commit, push, daily installation or parent-session restart was performed for
these changes. Physical HDR/TTY output, multiple physical outputs, complete
PipeWire/portal sessions and an actual capture-blocked layer during a closing
animation were not runtime-tested. Renderer tests and source review do not replace
those acceptance checks or establish an unmeasured performance improvement.

## Official upstream integration, 2026-10-20

The personal branch integrates official niri through
`f51d761c456d635084b2f117a9ccbe9b96ad5aca` (51 upstream commits after the
shared `5f4469b6` base). Its adjacent Smithay checkout is pinned to
`452641cd981cfc2146a56fc38592ba8f4c592b99`, merging the previous custom
`a6a4be6d4` with official `8a270232f`. The existing `personal/smithay`
development checkout was not changed. [versions.env](versions.env) records the
new source boundary.

The merge retains HDR, Vulkan, Glass, smooth corners and minimized windows.
Gamma-control updates deferred while the HDR post-blend encoder owns the LUT now
use the upstream `GammaProps` modes. Pending resets and installed LUT, legacy or
CTM adjustments continue to block post-blend encoding. Screenshot capture and UI
opening both retain renderer-generic dispatch; IME popup overflow retains the
minimized-window lookup fallback.

Completed checks:

- `cargo build --locked --workspace`: passed, including the visual-test executable.
- Locked feature checks passed with no default features, individually with `dbus`,
  `systemd`, `dinit` and `xdp-gnome-screencast`, with default features, and with
  `profile-with-tracy`.
- `NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 cargo test --locked --workspace
  --exclude niri-visual-tests`: 318 passed, one existing optional Glass comparison
  ignored. This includes actual GLES/Vulkan renderer execution, not only compilation.
- `cargo clippy --locked --workspace --all-targets`: completed successfully with
  lint warnings; no automatic lint fixes were applied.
- The built `target/debug/niri` validated `glass.kdl`, `glass-test-preset.kdl`,
  `hdr-dp-1.kdl`, `smooth-corners.kdl` and `smooth-hdr-dp-1.kdl`.
- An isolated nested compositor on `/dev/dri/renderD128` exercised Glass and smooth
  corners, fullscreen entry/exit, black-to-transparent fullscreen backdrop reload,
  overview, custom minimize/restore, touchpad event-stream transitions and the
  screenshot UI with omitted optional IPC fields. It exited with status 0.
  Fullscreen backdrop screenshots differed at 3,993,566 pixels.

Runtime evidence is under
`personal/validation/upstream-20261020/runtime-confirmed/` in this checkout:
`result.json`, `nested.log` and five PNG captures. The first run completed the
behavior checks but its temporary driver incorrectly expected an IPC reply after
Quit; the confirming run handles shutdown without that assumption.

`cargo +nightly fmt --all -- --check` found formatting debt in nine files that also
fail formatting at the pre-merge HEAD. The new gamma predicate follows rustfmt's
output; unrelated code was not reformatted.

Independent source review found and resolved the stale gamma-state field access.
It also identified pre-existing behavior outside this merge: resume applies gamma
without the HDR ownership guard, minimized-strip clicks can restore windows behind
the lock screen, and an already-started swipe can survive lock/screenshot entry.
Those paths were not changed or claimed fixed.

No daily installation, parent-session restart, physical HDR/VRR test or hardware
suspend/resume acceptance was performed. Nested output does not support VRR, so
the new fullscreen-only rule was parsed and integrated but not validated on a
physical variable-refresh output. Earlier human acceptance records below apply
to their recorded builds, not to this candidate.

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
| Real TTY HDR10 video trial | Passed user visual checklist; actual HDR signalling and player target readback confirmed | `validation/hdr-video/acceptance.json`; user follow-up, 2026-10-09 |

The final workspace command was:

```sh
NIRI_TEST_REQUIRE_GPU=1 LIBGL_ALWAYS_SOFTWARE=1 cargo test --all --locked
```

GLES execution used Mesa 26.2.4 llvmpipe (LLVM 23.1.1), OpenGL ES 3.2. This is an actual software renderer executing the GLSL, not a claim of physical GPU or HDR display validation. Vulkan's existing rendering/fallback checks used the available default ICD; the complete Vulkan desktop was not tested. A trial specifying a nonexistent lavapipe ICD failed before rendering; removing that invalid override produced the passing final result.

The six targeted Glass tests exercise observable risks: lazy shader compilation with assertions that the Glass program actually drew; neutral Glass matching the ordinary background; HDR PQ readback against an independent conversion expectation at 203 cd/m² reference white; restoration of a custom SDR reference state; translucent input/rounded-edge premultiplication; refraction modes and translated-window geometry; disabled Glass without shader use; Vulkan and non-xray fallback retaining ordinary effects; and real Wayland wallpaper color metadata rejecting HDR input while retaining blur. Existing renderer scenes include overview/zoom. Configuration tests cover defaults, parameter/rule merging, independent `off`/`off false`, invalid values and unknown/ineffective options.

A separate nested release compositor used its own Wayland/IPC sockets and the GTK demo. At 1871×2135, captured SDR screenshots changed 2,387,940 pixels with Glass enabled/disabled and matched exactly after restoring the enabled configuration. Overview and fullscreen toggles completed; an unsupported non-xray request emitted one warning for its category, and the nested compositor exited with status 0. Evidence is in `validation/nested-result.json`, `nested.log` and the PNG captures. This validates ordinary nested SDR rendering and actual visible Glass execution. It does not establish physical HDR output or HDR screenshot/screencast colors.

Initial focused-window captures hid all background effects behind the base's filled focus ring. A reviewer traced this existing policy in the tile/focus-ring code; setting `draw-border-with-background false` in the example resolved it without a renderer change. The GTK demo avoids a separate wallpaper tool and terminal dependency. Foot-specific effect behavior was not revalidated after this configuration correction.

A read-only reviewer inspected the port for renderer and color-path failures; reviewer inspection is not independent test execution. Passing these checks does not establish HDR screenshots, screencasts, direct scanout transitions, perceptual HDR correctness or performance improvements.

Initial user follow-up (2026-10-09): the user reported that the basic TTY trial had no problems. A later explicit HDR10 video acceptance checked the display's HDR indication, video colors, fullscreen transitions and return to desktop/Glass, and the user replied that it passed. The corresponding real `/dev/tty3` logs were then inspected: DP-1 HDR signalling switched on/off successfully, the player target was 10-bit `rgb10a2` BT.2020/PQ, and both console runs exited with status 0. Evidence is retained in `validation/hdr-video/tty-lnkiauar/`, `tty-qhspa_na/` and `acceptance.json`. This combines human observation with actual DRM-path logs; it is not a measured display calibration or an agent-run physical visual test. Screenshot, screencast and multiple-output acceptance remains pending.

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

## Launcher spawn isolation

The captured desktop trace identified two roughly 300–350 ms forks per launch and
a main-thread allocator wait overlapping the first fork. The fix moves application
creation into a freshly executed private helper, leaving the compositor-side launch
eligible for Rust's `posix_spawn` path.

`cargo test --locked -p niri --test spawning` passes. Its isolated process boundary
checks non-UTF-8 and empty arguments, environment precedence/removal, a request larger
than pipe capacity, activation tokens, shell expansion, restored nofile limits and
unblocked signals. Missing/non-executable targets and malformed requests must terminate
with failure instead of leaving a stuck helper. The registered `pthread_atfork`
handler remains uncalled in the compositor-side process on this Linux host; this
assertion is not a trace of every possible raw process-creation syscall.

A throwaway debug-build harness compared the previous spawning implementation with
the fixed implementation in the same process, touching anonymous memory with
transparent huge pages disabled. Three launches per implementation and size measured
time from the spawn call to the child's Unix-datagram notification:

| Resident test allocation | Previous median | Fixed median |
| --- | --- | --- |
| 64 MiB | 3.59 ms | 1.98 ms |
| 1024 MiB | 55.82 ms | 2.41 ms |

The actual `target/debug/niri --internal-spawn` path also passed checks for systemd
scope membership, application survival after helper exit, inherited working
directory, null standard streams, configured and absent PATH, and application-only
`LD_PRELOAD`. Missing/non-executable programs returned status 1 without hanging;
immediate application exit and an unavailable session bus returned successfully.
Local evidence: `/tmp/niri-spawn-fixed-05k07_h2/final-results.json`. The temporary
benchmark source was removed.

These are process-launch measurements, not keyboard-to-visible-window latency.
The active desktop and deployed release were not replaced. Compositor heap growth
remains a separate unresolved issue.

The spawning test also passes with `--no-default-features`, and the scoped nightly
rustfmt check passes. `cargo test --locked --all` stopped in the niri library suite:
323 passed, 2 failed and 1 ignored. The failures were
`tests::smooth_corners::soft_shadow_contours_match_accepted_rendering` and
`tests::smooth_corners::thick_inner_border_and_oversized_shadow_keep_valid_smooth_geometry`.
These shadow-rendering tests were not changed by this fix; the workspace gate remains
failed, and later workspace suites did not run.

On 2026-11-11, the user requested deployment (`部署先`). The default-feature release
was published as an authorized daily trial, SHA-256
`acceafb71ac0383d58966cb18f3850a3fc6bc8c20ac0325ddeabfd7674bd2dc3`.
Its five personal example configurations, release helper/scope smoke, live configuration
validation and 52 deployment/migration tests passed. This is an uncommitted working-tree
build, not a clean revision-only build. The previous daily selection remains available
for rollback; the active compositor stayed at PID 5221 on the previous executable.
Acceptance, source snapshot and deployment readback are retained under
`personal/validation/launcher-spawn-20261111T083224Z/`. The new build takes effect at
the next personal-session login; the failed shadow tests and visual-acceptance limits
above remain unchanged.

## Manual observations still required

### Prepared local HDR video acceptance

The user supplied `/home/hewel/Data/jellypilot/test-videos`. The selected HDR10 source is a 3840×2160 HEVC 10-bit episode tagged BT.2020/PQ with mastering metadata (1000 cd/m²), MaxCLL 960 and MaxFALL 803; these describe the video, not the display. Its full path and ffprobe records are retained privately in `validation/hdr-video/`, outside the committed source tree. A separate Dolby Vision profile 8 source was identified but is not the first acceptance fixture.

From TTY 3, use the short entry:

```sh
cd ~/Codes/niri/personal
./tty video
```

The player starts windowed with compositor HDR mode `auto`. Focus the video and press **f** to enter/leave fullscreen. In auto mode, HDR should engage for the fullscreen HDR surface and disengage when returning to the windowed SDR desktop. Press **F1** for always-on HDR, **F2** for auto, **F3** to remove the HDR node and disable it. OSD reports the requested mode; verify the actual mode using the display's menu and the retained compositor log. **Shift+i** shows mpv statistics. Use arrow keys to seek, and **Super+Shift+E** to exit the trial, then **Ctrl+Alt+F2** to return to the original desktop.

Observe natural skin tones, shadows and bright highlights, absence of pink/green casts or washed-out colors, smooth fullscreen transitions and correct desktop/Glass colors after leaving fullscreen. With F1 and a windowed video, check SDR wallpaper Glass on the HDR output as well. Only a human report of the real-display result can satisfy the acceptance gate for daily setup.

Each run creates `validation/hdr-video/tty-*/` with `niri.log`, `mpv.log`, actual player arguments, source metadata, mode requests, player source/target color properties, fullscreen/hardware-decoder/drop-frame state and exit codes. No `--session` is used and the trial runs on a private D-Bus session. The default GLES compositor is retained; mpv itself uses its Wayland Vulkan output with color-management hints. Its user config is bypassed for this controlled check. Relevant primary documentation: [mpv 0.41 stable color hints](https://mpv.io/manual/stable/#options-target-colorspace-hint).

A nested player preflight completed using the chosen video, with 10-bit `rgb10a2`, BT.2020/PQ target readback, hardware decoding and windowed/fullscreen/windowed transitions. The compositor exited with status 0. Evidence: `validation/hdr-video/tty-gkne4xdz/`. That run began at 120 seconds; subsequent trials start at the beginning to support short test clips too. A second preflight verified all three mode keys, watched config reloads, valid player drop-frame properties and player/compositor exit status 0; evidence: `validation/hdr-video/tty-xw7m6lxc/` and `preflight.json`. These nested runs verify player integration. The subsequent real TTY video acceptance and observations are recorded above, separately.

| Scenario | Procedure | Expected observation |
| --- | --- | --- |
| Ordinary SDR desktop | Run the SDR example; move/resize windows, open overview and toggle fullscreen | Stable wallpaper/window colors and rounded edges; refraction follows the window rather than drifting with its screen position |
| Glass disabled | In a trial copy, add `off` to the Glass block and reload; then use `off false` | Ordinary blur remains; enabling Glass adds refraction/tint; disabling it restores ordinary rendering |
| SDR wallpaper on HDR output | Run the HDR example with the same static background | Glass remains visible; no double conversion, washed-out SDR background, unexplained brightness jump or colored transparent-edge fringe |
| HDR enable/disable | Compare trial copies with `hdr mode="on"` and no `hdr` node (this version has no `mode="off"`) | Display mode transitions complete and both modes render correctly; assess real brightness rather than assuming encoded pixels prove it |
| HDR fullscreen entry/exit | In a trial copy use `mode="auto"`; play a known HDR source in a color-management-capable player/game, enter/leave fullscreen | Correct HDR activation and return to SDR; compositor effects/color conversion remain present when composition is required |
| Screenshot colors | Capture SDR, HDR-content and wallpaper/Glass scenes with the custom compositor; inspect exported files in a color-aware viewer | Files have the base compositor's intended color representation without unexpected tint/clipping; compare with the unmodified build if uncertain |
| Screencast colors | Set up the appropriate portal in an isolated test session, select the test output/window, and record SDR and HDR scenes | Captured colors match the base compositor's intended screencast conversion; confirm in a viewer, not just that a stream starts |
| Unsupported non-xray Glass | Use `xray false` with both blur and Glass | One diagnostic for the category; ordinary blur still works; no globally forced xray behavior |
| HDR/linear/wide-gamut wallpaper input | Supply a background client declaring those color spaces | Glass is disabled only for the unsupported background with a deduplicated warning; ordinary effects remain |
| Mixed SDR/HDR outputs | Connect an additional output and move the same window between outputs, including fullscreen transitions | Per-output colors/effects remain correct and no stale cached color-space conversion appears |

The simple demo does not provision a full portal/service environment, generate an HDR video source, or prove screencast correctness. Use known content and a properly configured isolated session for those checks. Beyond the accepted HDR10 video/transition/desktop checks, screenshot, screencast, multiple-output and unsupported-input scenarios remain pending; do not infer them from the video trial, nested rendering or shader readback.
