# Daily desktop with the existing niri configuration

The personal session reads the single existing entry point
`~/.config/niri/config.kdl`. Select **Niri HDR + Glass** in GDM. The system-provided
Niri binary and session entry remain installed.

An **accepted build** is tied to the binary's SHA-256 and the user's stated validation
scope. The **daily build** is selected for the next login; the **running build** can
remain an older version until logout. The **rollback build** is the preceding accepted
custom build. An installation or rollback never logs out or restarts the desktop.

The accepted smooth-corner render build is
`0d489c96f6c74b1ad5992a80298eafac3f97ac0e`, SHA-256
`b296f3dce6fa779d114058c4be1ba84a0e23a3ac9ed2062e1c24d6b6ca9bbc3b`.
Its corner acceptance authorizes daily trial; physical HDR10 acceptance remains tied
to the earlier build. Acceptance does not extend automatically to later recompilations,
capture, or multi-output color checks.

## Install, update and retry

From `~/Codes/niri/personal/niri`, select an artifact and its matching acceptance
record explicitly:

```sh
scripts/personal-install-daily.sh install \
    --artifact ../artifacts/niri-hdr-glass-smooth \
    --acceptance ../validation/corner-aa/daily-acceptance.json
scripts/personal-install-daily.sh status
```

Use the same command for the first installation, updates and retries. The old
`--install`, `--install-smooth` and `--install-unified` modes have been retired.
There is no implicit build, branch fetch, version selection or config rewrite.
The command is safe to run while a desktop is active: it validates the selected
build, publishes its files, and reloads unit metadata for the next login.

Acceptance must contain `result: "passed"`, `source: "user-observed"` and the exact
`binary_sha256`. Preserve the original description of what the user actually checked;
never create acceptance by copying a hash from a newer build. The artifact and initial acceptance record are copied together before configuration
validation, so replacing either caller path cannot change the validated build.
Authentication occurs only after those staged bytes pass validation; a separately
passed plan digest binds the privileged copy to that exact prepared selection. A cancelled prompt
leaves the selection and configuration unchanged; retry the same command.

Daily deployment stores immutable bundles under
`/usr/local/lib/niri-personal/releases/<bundle-id>/`. A bundle contains the executable,
session and one-time migration helpers, acceptance record, and content manifest.
`selection.json` atomically selects the daily and rollback bundles after complete
copies are verified. A login pins one bundle for migration, validation and compositor
startup, even if a different build is selected in the meantime. Already executing
files are not truncated. Old bundles are retained; there is no automatic deletion.

On the first run against the earlier flat installation, its accepted binary is imported
as a rollback build using the current session helpers. If it differs from the selected
artifact, supply its actual acceptance record with `--previous-acceptance PATH`.
An unrecorded installed binary is never silently treated as accepted. If both binaries
and their bundle files are identical, no redundant rollback copy is invented.
The earlier flat files are retained for interrupted-bootstrap recovery. A legacy
wrapper already running during bootstrap has no bundle pin; its eventual service
handoff uses only the retained flat executable, verified against the accepted SHA
recorded at import. This narrow compatibility path also checks the retained wrapper
hash and never substitutes the newly selected build. New logins always pin a bundle.
An unpinned direct service start without this verified legacy context is refused;
start daily sessions through the login entry.

The privileged adapter writes only the personal bundle store and these entry files:
`/usr/local/bin/niri-personal-session`,
`/usr/local/lib/systemd/user/niri-personal.service`,
`/usr/local/lib/systemd/user/niri-personal-shutdown.target`, and
`/usr/share/wayland-sessions/niri-personal.desktop`. It never executes a staged binary
as root. The ordinary user performs configuration validation.

## Configuration and one-time migration

Maintain personal settings in the existing files:

| File | Custom settings |
| --- | --- |
| `cfg/outputs.kdl` | `hdr mode="auto"` in the existing DP-1 / HDMI-A-1 blocks; preserve modes, scaling and positions |
| `cfg/layout.kdl` | Existing radius 12 with `smoothing=0.65` |
| `cfg/rules.kdl` | Existing five blur-only window groups gain xray Glass; Noctalia/anyrun non-xray effects retained |

Binary deployment and configuration migration are separate. The existing reviewed
`~/.config/niri/.pending-personal` directory is preserved. A `prepared` migration is
validated using its candidate; an `applied` migration validates the live canonical
config. Updates never copy or apply the pending files again.

At login, the selected session helper first refuses a running graphical session.
Then it applies a prepared migration once, checking the originals and candidate,
validates the result, and starts the normal niri `--session` integration. Subsequent
logins leave later user edits intact. If source files were edited after preparation,
login stops with a diagnostic instead of replacing those edits.

For a first migration on this specific host, prepare directly in the canonical config
folder, before installing:

```sh
scripts/personal-unify-config.py prepare \
    --binary ../artifacts/niri-hdr-glass-smooth
```

This deliberately checks the reviewed DP-1/HDMI-A-1 blocks, global radius 12 and five
blur groups; it is not a general KDL migration tool. An existing stage is never
overwritten. Preserve and inspect any existing pending directory before preparing a
replacement. Keep `original/` for the system-provided Niri fallback.

If preparation itself failed before producing `manifest.json`, preserve that incomplete
directory elsewhere and prepare again. If a manifest says `applying`, retry a personal
login from the login screen: the helper recovers an interrupted write only when files
still contain its known old/new bytes. If subsequent user edits exist, it stops and
requires manual reconciliation with `original/` and `candidate/`. Updates refuse an
unresolved `applying` state; they do not guess how to recover configuration.

Historical configuration/deployment evidence remains under `personal/daily/`, including
`unified-installation.json`, `installation.json` and `smooth-installation.json`.
Inactive old overlays may remain as backups; the selected session does not read them.

## Roll back an accepted custom build

```sh
scripts/personal-install-daily.sh rollback
```

This validates the rollback executable against the current config (or the still
prepared migration candidate), then selects it for the next login. It preserves all
current configuration files. If the older build cannot read them, selection is refused:
review the diagnostic and make a deliberate compatible configuration edit, or retain
the current build. No saved configuration is restored automatically. Repeating rollback
switches between the last two selections.

If publishing was interrupted, rerun the original install command. Complete bundles
are reused and verified, incomplete copies are removed under the deployment lock, and
a changed selection causes a clear retry diagnostic. A failure of `daemon-reload`
occurs after selection; rerun installation to retry metadata reload without losing the
rollback selection. No service is automatically started or restarted.

## Returning to the system-provided Niri

The original `/usr/bin/niri` cannot parse `hdr`, `liquid-glass` or `smoothing`.
This is a separate fallback from the accepted custom rollback. After backing up newer
edits, restore or manually merge the three original files before selecting **Niri**:

```sh
cd ~/.config/niri
backup=$(mktemp -d ~/Codes/niri/personal/daily/before-config-rollback-XXXXXXXX)
cp -a cfg "$backup/"
for name in outputs layout rules; do
    cp -a ".pending-personal/original/cfg/$name.kdl" "cfg/$name.kdl"
done
/usr/bin/niri validate -c config.kdl
```

Merge newer edits from the backup while removing unsupported custom settings.
The completed migration remains marked applied, so a later personal login does not
silently restore custom settings.

## Validation

Run `python3 scripts/test-personal-deploy.py`. It exercises initial installation,
accepted updates, repeated retries, privilege failure, importing the old flat layout,
interrupted publication, stale selection, staged-content or plan tampering, caller-path replacement during validation, legacy
login handoff during bootstrap, config preservation
and incompatible rollback through the deployment module using temporary filesystems.
The prior ten migration recovery/edit-preservation cases are now reproducible in this
same tracked suite. Tests do not request root or install session files. Real GDM login
and policy authentication remain separate host checks.
