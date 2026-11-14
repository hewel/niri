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

Observed acceptance contains `result: "passed"`, `source: "user-observed"` and the exact
`binary_sha256`. If the user explicitly requests installing a candidate before human
validation, record `result: "approved-for-daily-trial"`, `source: "user-authorized"`,
the exact `binary_sha256`, their `user_request`, and a `scope` distinguishing completed
checks from unverified ones. This requires an actual user instruction; automated
checks alone never authorize installation. Status preserves this distinction.
Never claim observed success without a new user observation of that exact hash,
or copy acceptance to a newer hash.
The artifact and initial acceptance record are copied together before configuration
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

Updating only the acceptance record creates a new immutable bundle but preserves
the existing rollback selection. Runtime identity compares the verified contents of
`niri`, `session.sh` and `unify-config.py`, not acceptance metadata. Changing any of
those three files is a runtime update. If there was no rollback build, a metadata-only
update does not invent one. Old evidence bundles remain on disk.

Published store, releases and bundle directories have explicit `0755` permissions,
independent of the publisher's umask. Missing parent directories created by deployment
also receive `0755`; existing general system directories keep their administrator-set
permissions. There is no recursive permission relaxation. Private staging directories
remain private, and symlink aliases for the dedicated deployment directories are refused.

On the first run against the earlier flat installation, its accepted binary is imported
as a rollback build using the current session helpers. If it differs from the selected
artifact, supply its actual acceptance record with `--previous-acceptance PATH`.
An unrecorded installed binary is never silently treated as accepted. If the imported
legacy build and candidate have identical runtime contents, differing acceptance records
do not create a duplicate rollback selection. The legacy evidence bundle is retained.
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
login from the login screen: the helper recovers an interrupted write only when live
and retained files still contain its known old/new bytes. If user edits or uncertain
recovery evidence exist, it stops before service startup and requires manual
reconciliation. Updates refuse an unresolved `applying` state; they do not guess how
to recover configuration. Existing prepared/applying manifests remain usable.

All forward, recovery and rollback writes use Linux `renameat2(RENAME_EXCHANGE)`.
Before exchange, the helper creates a private `recovery/replace-*/` directory inside
the stage with a replacement file named `saved` and an `intent.json` containing the
target path, replacement inode and allowed content hashes. Exchange atomically puts
the replacement at the live path and moves the actual displaced inode into `saved`.
There is no missing-path interval. Unsupported exchange or a stage on a different
filesystem fails safely; there is no overwrite-based fallback.

After a process interruption, the saved inode identifies whether exchange occurred.
An unexchanged replacement is not treated as a displaced user file. Unedited
interruptions continue automatic recovery. Files displaced by any completed exchange
are retained even after success, including writes an editor later makes through an
already-open file descriptor. Retained content is checked on subsequent apply/login,
even when the main manifest is already `applied`. The migration lock serializes only
migration processes; it does not assume editors cooperate. These are process-crash
guarantees, not a claim of power-loss durability.

A save racing with exchange may be in `recovery/replace-*/saved`, rather than at the
live path. The helper stops when it detects that conflict; it does not overwrite it
again to attempt automatic rollback. Inspect each `intent.json` to identify its
target, and compare the saved file with the live config, `original/` and `candidate/`.
Do not delete recovery evidence or change journal states to force a retry. Preserve
the whole stage before manual reconciliation. Once the intended live configuration
has been reconciled and validated with the selected accepted binary, retain the old
stage elsewhere instead of leaving it at `.pending-personal`; login then validates
the reconciled live config without replaying that migration. Do not reprepare an
already-migrated config blindly.

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
switches between the last two selections. Rollback changes no system entry files and
does not run `systemctl --user daemon-reload`; it needs no user-manager connection
for metadata reload.

If publishing was interrupted, rerun the original install command. Complete bundles
are reused and verified, incomplete copies are removed under the deployment lock, and
a changed selection causes a clear retry diagnostic.

Installation reloads user-unit metadata only after publication succeeds. If this
post-selection step fails, the command exits nonzero but explicitly reports the build
already selected and its rollback selection: the version switch has not been undone.
Retry the same install command with unchanged artifact and acceptance files once the
user manager is available; this preserves the rollback selection. Do not use `rollback`
to retry a failed metadata reload. No service is automatically started or restarted.

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
login handoff during bootstrap, config preservation, restrictive-umask publication,
metadata-only acceptance updates, runtime-helper updates, post-selection reload failures
and incompatible rollback through the deployment module using temporary filesystems.
The migration cases cover old journals, deterministic process interruption before
and after exchange, interruption during recovery, editor saves between recovery
writes and at the final replacement boundary, late saves through old file descriptors,
unsupported exchange, and real session-shell refusal on conflict. Tests do not request
root or install session files. Real GDM login and policy authentication remain separate
host checks.
