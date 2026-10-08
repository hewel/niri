# Daily desktop setup after HDR video acceptance

The current setup is staged under `personal/daily/`; it has not been installed or activated. A user-observed HDR video pass must be recorded in `personal/validation/hdr-video/acceptance.json` before `scripts/personal-install-daily.sh --install` proceeds. That record must identify the accepted binary SHA-256, so acceptance cannot accidentally authorize a different build. The installer does not generate a pass. Installation needs administrator permission for the GDM entry and separate binary/service files.

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

After the first personal login, verify the service executable/config, Noctalia, wallpaper, application shortcuts, Glass, HDR fullscreen entry/exit and portal capture. GDM integration and these real-session checks remain pending while the setup is staged. The `DesktopNames=niri` field follows the installed niri/GDM session entry convention; the generic desktop-file validator flags it in both the stock and personal entries as a nonstandard key.

Preparation checks passed: candidate config parsing, shell syntax, refusal of the running original graphical session without environment import, and installer refusal before any write when acceptance is absent. A read-only reviewer identified the output first-match requirement and the autostart target ordering; both were addressed. `systemd-analyze --user verify` passed on copies of the units with binary/config paths substituted to the current staged files. Verifying the exact install paths currently reports the expected missing `/usr/local/lib/niri-personal/niri`; repeat that check after installation. No service was started or stopped by these checks.
