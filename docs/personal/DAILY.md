# Daily desktop setup after HDR video acceptance

The daily setup was installed on 2026-10-09 after the user's explicit real TTY HDR10 video acceptance. Installation evidence is in `personal/daily/installation.json`; the current desktop was not switched. A user-observed HDR video pass must be recorded in `personal/validation/hdr-video/acceptance.json` before `scripts/personal-install-daily.sh --install` proceeds. That record must identify the accepted binary SHA-256, so acceptance cannot accidentally authorize a different build. The installer does not generate a pass. Installation uses cached sudo authorization or a single system Polkit authentication for the GDM entry and separate binary/service files.

The candidate config is generated from `examples/personal/daily.kdl.in`, replacing `@USER_CONFIG@` with the quoted absolute path of the user's existing config. It includes that config so existing binds, autostarts, application rules and future ordinary config edits remain live. The original config is not changed. The first DP-1 block preserves the detected host's mode, scale, position, startup focus and hot corner while adding HDR `auto`. **Output blocks use first-match resolution**, so this complete override precedes the included original; synchronize it if the original output settings change. No HDR peak override is added.

Glass is enabled with xray only for the matched applications that already had blur rules. Client transparency is retained. Noctalia's non-xray rules retain their original background effects. Unsupported Glass combinations retain the port's documented fallback.

The installed files are:

| File | Purpose |
| --- | --- |
| `/usr/local/lib/niri-personal/niri` | Separate custom binary; `/usr/bin/niri` is retained |
| `/usr/local/bin/niri-personal-session` | Separate session wrapper, never invokes stock `niri-session` |
| `/usr/local/lib/systemd/user/niri-personal.service` | Absolute custom binary/config paths and normal notify/session integration |
| `/usr/local/lib/systemd/user/niri-personal-shutdown.target` | Stops the graphical session after the personal service ends |
| `/usr/share/wayland-sessions/niri-personal.desktop` | GDM choice named **Niri HDR + Glass** |
| `~/.config/niri-personal/config.kdl` | Independent config overlay |

Before importing any environment or starting/stopping services, the wrapper refuses an existing active/activating graphical session. The service clears the trial-only notification suppression variable and uses `--session` for normal portal/autostart integration. User D-Bus and graphical session targets are shared; these two daily sessions are alternatives, not parallel sessions. [systemd desktop session guidance](https://github.com/systemd/systemd/blob/main/docs/DESKTOP_ENVIRONMENTS.md)

The installer backs up the original config and any pre-existing files at these personal installation paths. It then copies the staged files and reloads unit definitions. It does not start/enable a service, restart GDM, log out, change the remembered session choice or switch VTs. Select **Niri HDR + Glass** in GDM on the next login. Select the original **Niri** entry to return to the stock binary and original configuration.

After the first personal login, verify Noctalia, wallpaper, application shortcuts, Glass, HDR fullscreen entry/exit and portal capture. The installed service executable/config paths are already verified; the first actual GDM login and its application/portal integration remain pending. The `DesktopNames=niri` field follows the installed niri/GDM session entry convention; the generic desktop-file validator flags it in both the stock and personal entries as a nonstandard key.

Preparation checks passed: candidate config parsing, shell syntax, refusal of the running original graphical session without environment import, and installer refusal before any write when acceptance is absent. A read-only reviewer identified the output first-match requirement and the autostart target ordering; both were addressed. `systemd-analyze --user verify` passed both on staged-path copies and on the exact installed paths after installation. The installed binary SHA-256 matches the accepted artifact; 13 original binary/session/config files retained their hashes. Stock `niri.service` remained active and `niri-personal.service` inactive. No service was started or stopped by these checks.

The original configuration backup is `/home/hewel/Codes/niri/personal/daily/backup-U4qvxm5O/original-niri-config`. To return to stock behavior, select **Niri** in GDM; there is no original configuration change to undo.

## Accepted smooth-corner daily upgrade

On 2026-10-09 the user accepted the corrected smooth-corner build
`0d489c96f6c74b1ad5992a80298eafac3f97ac0e` for daily trial. Its SHA-256 is
`b296f3dce6fa779d114058c4be1ba84a0e23a3ac9ed2062e1c24d6b6ca9bbc3b`.
This is visual corner acceptance; the earlier real HDR10 acceptance remains tied
to its original binary. New-build HDR video, capture and multiple-output color
acceptance are not inferred from the corner pass.

The explicit upgrade command is `scripts/personal-install-daily.sh --install-smooth`.
It requires the matching user-observed record in
`personal/validation/corner-aa/daily-acceptance.json`, validates the candidate, and
refuses an active personal service. The binary is replaced by renaming a new inode.
The service is switched to `~/.config/niri-personal/config-smooth.kdl` for the next
login; this overlay includes the existing `config.kdl`, so its current double-output
layout and locally maintained rules remain live. Neither the base personal config
nor the original Niri config is rewritten. The overlay sets window radius **12**
and `smoothing=0.65`, retaining the host's existing radius rather than the demo's 32.
Edit this independent overlay to change the daily window radius or smoothing.

Generate the staged overlay before installation:

```sh
python3 - <<'PY'
import json
from pathlib import Path
template = Path('examples/personal/daily-smooth.kdl.in').read_text()
config = str(Path.home() / '.config/niri-personal/config.kdl')
Path('../daily/config-smooth.kdl').write_text(template.replace('@PERSONAL_CONFIG@', json.dumps(config)))
PY
scripts/personal-install-daily.sh --install-smooth
```

Installation evidence is recorded separately in `personal/daily/smooth-installation.json`.
The upgrade was installed successfully: the installed binary matches the accepted
hash, its installed overlay parses, and `systemd-analyze --user verify` passes.
Seventeen original/base personal files retained their hashes; the original service
stayed active and the personal service inactive. Missing acceptance, mismatched
artifact and active personal session preflights refused before writes. An independent
source review checked executable replacement, old-config isolation and copied-byte
hash validation. These checks do not start a desktop or stand in for daily use.
The installer prints the backup directory containing the prior personal binary,
configuration and session files. Choose **Niri HDR + Glass** on the next GDM login.
No automatic logout or service restart occurs. The original **Niri** login remains
the immediate fallback. During daily use check the existing HDR video again, move
windows between the two outputs, and check screenshot/screencast colors; these are
distinct from the accepted corner appearance and automated rendering checks.
