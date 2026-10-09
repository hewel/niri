# Daily desktop with the existing niri configuration

The personal session uses `/usr/local/lib/niri-personal/niri` and the single existing
entry point `~/.config/niri/config.kdl`. There is no active `config-smooth.kdl` or
separate personal config overlay. Select **Niri HDR + Glass** in GDM at the next
login. The original binary and session files remain installed.

The user chose the custom version as the daily default on 2026-10-09. Its accepted
render build is `0d489c96f6c74b1ad5992a80298eafac3f97ac0e`, SHA-256
`b296f3dce6fa779d114058c4be1ba84a0e23a3ac9ed2062e1c24d6b6ca9bbc3b`.
The corner visual pass authorizes daily use; earlier physical HDR10 acceptance is
still tied to the earlier build. New-build video, capture and multi-output color
checks remain separate from corner acceptance.

Maintain the differences in the existing files:

| File | Custom settings |
| --- | --- |
| `cfg/outputs.kdl` | `hdr mode="auto"` inside each existing DP-1 / HDMI-A-1 block; existing modes, scale, position and hot corners retained |
| `cfg/layout.kdl` | Existing radius 12 gains `smoothing=0.65`; clipping and other layout settings retained |
| `cfg/rules.kdl` | Existing five blur-only window rule groups gain xray Glass with the accepted preset; Noctalia/anyrun non-xray rules retained |

There are no duplicated output overrides. Change connector names, resolution,
scaling and HDR in the same output blocks. Change radius/smoothing in the existing
global window rule. Change per-application Glass in the existing rules. The root
config, binds, autostarts, environment, overview and input files are reused verbatim.

Because the original compositor was active during preparation, the three changed
files are staged in `~/.config/niri/.pending-personal/candidate`. The session wrapper
applies them once, **after** refusing any running graphical session and **before**
importing environment or starting the personal service. It validates the candidate,
checks that the source config has not changed, applies only the three files, and
validates the result. Failures roll back, and interrupted writes are recovered only
if the files still contain the known old/new bytes. Later user edits are not reapplied
or overwritten on subsequent logins. If you edit the base config before the first
personal login, migration stops with a diagnostic and requires a freshly reviewed
preparation rather than replacing your edits.

The previous files are retained in `~/.config/niri/.pending-personal/original` and
`personal/daily/unified/original`. Installation evidence is in
`personal/daily/unified-installation.json`; older overlay deployment evidence remains
in `personal/daily/installation.json` and `smooth-installation.json`. Old personal
config files may remain as inactive backups; the installed service does not read them.

To prepare this host-specific migration from the worktree (explicitly reviewed
DP-1/HDMI-A-1 blocks, radius 12 and five blur groups):

```sh
scripts/personal-unify-config.py prepare --stage-dir ../daily/unified
scripts/personal-install-daily.sh --install-unified
```

An existing stage/pending directory is never overwritten. Preserve it before preparing
a replacement. This is a one-time config migration, not an unattended updater.
The installer still verifies the user-observed acceptance hash, checks the copied
binary again after authentication, backs up installed files, and reloads unit
metadata without restarting, logging out, or switching desktops. Legacy `--install`
and `--install-smooth` options are retained for reproducing the earlier staged setups.

The installed service, wrapper and separate binary retain normal niri `--session`,
notify, portal and autostart integration. The wrapper refuses an active graphical
session before making config or user-manager changes. A first GDM launch with the
unified config remains a user-operated check; no desktop is launched by validation.

## Returning to the original binary

The original `/usr/bin/niri` does not parse `hdr`, `liquid-glass` or `smoothing`.
Selecting **Niri** requires restoring compatible config first. After daily use,
back up any newer changes, then restore the three original files:

```sh
cd ~/.config/niri
backup=$(mktemp -d ~/Codes/niri/personal/daily/before-config-rollback-XXXXXXXX)
cp -a cfg "$backup/"
for name in outputs layout rules; do
    cp -a ".pending-personal/original/cfg/$name.kdl" "cfg/$name.kdl"
done
/usr/bin/niri validate -c config.kdl
```

Then log out and select **Niri**. Other config files are retained. If you have edited
these three files during daily use, merge those edits from the rollback backup after
removing only the unsupported custom settings. The completed migration remains
marked applied, so subsequent personal logins do not silently restore custom settings.
