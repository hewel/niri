use std::ffi::{OsStr, OsString};
use std::io::{BufReader, BufWriter, Write as _};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use std::{io, thread};

use anyhow::Context as _;
use atomic::Atomic;
use libc::{getrlimit, rlim_t, rlimit, setrlimit, RLIMIT_NOFILE};
use niri_config::Environment;
use serde::{Deserialize, Serialize};
use smithay::wayland::xdg_activation::XdgActivationToken;

use crate::utils::{expand_home, IS_SYSTEMD_SERVICE};

pub static REMOVE_ENV_RUST_BACKTRACE: AtomicBool = AtomicBool::new(false);
pub static REMOVE_ENV_RUST_LIB_BACKTRACE: AtomicBool = AtomicBool::new(false);
pub static CHILD_ENV: RwLock<Environment> = RwLock::new(Environment(Vec::new()));
pub static CHILD_DISPLAY: RwLock<Option<String>> = RwLock::new(None);

static ORIGINAL_NOFILE_RLIMIT_CUR: Atomic<rlim_t> = Atomic::new(0);
static ORIGINAL_NOFILE_RLIMIT_MAX: Atomic<rlim_t> = Atomic::new(0);

const SPAWN_HELPER_ARG: &str = "--internal-spawn";

#[derive(Serialize, Deserialize)]
struct SpawnRequest {
    program: OsString,
    args: Vec<OsString>,
    environment: Vec<(OsString, Option<OsString>)>,
    nofile_soft: rlim_t,
    nofile_hard: rlim_t,
    systemd_scope: bool,
}

/// Handles the private re-exec mode before compositor initialization or environment changes.
///
/// The helper creates a detached application with a fresh address space and remains alive
/// until its systemd scope is ready. It must not recursively call `spawn`.
#[doc(hidden)]
pub fn run_spawn_helper() -> Option<anyhow::Result<()>> {
    let mut args = std::env::args_os();
    if args.nth(1).as_deref() != Some(OsStr::new(SPAWN_HELPER_ARG)) {
        return None;
    }
    Some(spawn_from_helper())
}

fn spawn_from_helper() -> anyhow::Result<()> {
    let request: SpawnRequest = serde_json::from_reader(BufReader::new(io::stdin().lock()))
        .context("error reading spawn helper request")?;
    let mut process = Command::new(&request.program);
    for (key, value) in request.environment {
        if let Some(value) = value {
            process.env(key, value);
        } else {
            process.env_remove(key);
        }
    }
    process
        .args(request.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    ORIGINAL_NOFILE_RLIMIT_CUR.store(request.nofile_soft, Ordering::SeqCst);
    ORIGINAL_NOFILE_RLIMIT_MAX.store(request.nofile_hard, Ordering::SeqCst);
    IS_SYSTEMD_SERVICE.store(request.systemd_scope, Ordering::Relaxed);
    crate::utils::signals::unblock_all().context("error unblocking helper signals")?;
    unsafe {
        process.pre_exec(|| {
            restore_nofile_rlimit();
            Ok(())
        });
    }
    let child = process
        .spawn()
        .with_context(|| format!("error spawning {:?}", request.program))?;
    // The helper itself keeps the scope non-empty if the application exits immediately.
    // Unlike the old intermediate child, it can handle exec errors without a pipe handshake
    // that deadlocks Command::spawn's error-path waitpid.
    #[cfg(feature = "systemd")]
    if let Err(err) = systemd::start_systemd_scope(&request.program, std::process::id(), child.id())
    {
        trace!("error starting systemd scope for spawned command: {err:?}");
    }
    // Do not wait for the application: exiting reparents it, and the compositor reaps us.
    drop(child);
    Ok(())
}

fn spawn_via_helper(request: &SpawnRequest) -> io::Result<Child> {
    // Re-exec the running image, even if its installation has since been replaced or removed.
    #[cfg(target_os = "linux")]
    let mut helper = Command::new("/proc/self/exe");
    #[cfg(not(target_os = "linux"))]
    let mut helper = Command::new(std::env::current_exe()?);

    helper
        .arg(SPAWN_HELPER_ARG)
        .stdin(Stdio::piped())
        .stdout(Stdio::null());
    // No pre_exec, uid/gid changes or PATH search: keep Rust's posix_spawn fast path.
    // In particular, never fall back to spawning the application here on helper failure.
    let mut child = helper.spawn()?;
    // Keep application overrides out of both argv (secrets/ARG_MAX) and the helper's
    // environment (LD_PRELOAD/LD_LIBRARY_PATH must only affect the application).
    let mut stdin = BufWriter::new(child.stdin.take().unwrap());
    let sent = serde_json::to_writer(&mut stdin, request)
        .map_err(io::Error::other)
        .and_then(|()| stdin.flush());
    // The helper validates the entire request through EOF before launching anything.
    drop(stdin);
    if let Err(err) = sent {
        let _ = child.wait();
        return Err(err);
    }
    Ok(child)
}

/// Increases the nofile rlimit to the maximum and stores the original value.
pub fn store_and_increase_nofile_rlimit() {
    let mut rlim = rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    if unsafe { getrlimit(RLIMIT_NOFILE, &mut rlim) } != 0 {
        let err = io::Error::last_os_error();
        warn!("error getting nofile rlimit: {err:?}");
        return;
    }

    ORIGINAL_NOFILE_RLIMIT_CUR.store(rlim.rlim_cur, Ordering::SeqCst);
    ORIGINAL_NOFILE_RLIMIT_MAX.store(rlim.rlim_max, Ordering::SeqCst);

    trace!(
        "changing nofile rlimit from {} to {}",
        rlim.rlim_cur,
        rlim.rlim_max
    );
    rlim.rlim_cur = rlim.rlim_max;

    if unsafe { setrlimit(RLIMIT_NOFILE, &rlim) } != 0 {
        let err = io::Error::last_os_error();
        warn!("error setting nofile rlimit: {err:?}");
    }
}

/// Restores the original nofile rlimit.
pub fn restore_nofile_rlimit() {
    let rlim_cur = ORIGINAL_NOFILE_RLIMIT_CUR.load(Ordering::SeqCst);
    let rlim_max = ORIGINAL_NOFILE_RLIMIT_MAX.load(Ordering::SeqCst);

    if rlim_cur == 0 {
        return;
    }

    let rlim = rlimit { rlim_cur, rlim_max };
    unsafe { setrlimit(RLIMIT_NOFILE, &rlim) };
}

/// Spawns the command to run independently of the compositor.
pub fn spawn<T: AsRef<OsStr> + Send + 'static>(command: Vec<T>, token: Option<XdgActivationToken>) {
    let _span = tracy_client::span!();

    if command.is_empty() {
        return;
    }

    // Spawning and waiting takes some milliseconds, so do it in a thread.
    let res = thread::Builder::new()
        .name("Command Spawner".to_owned())
        .spawn(move || {
            let (command, args) = command.split_first().unwrap();
            spawn_sync(command, args, token);
        });

    if let Err(err) = res {
        warn!("error spawning a thread to spawn the command: {err:?}");
    }
}

/// Spawns the command through the shell.
///
/// We hardcode `sh -c`, consistent with other compositors:
///
/// - https://github.com/swaywm/sway/blob/b3dcde8d69c3f1304b076968a7a64f54d0c958be/sway/commands/exec_always.c#L64
/// - https://github.com/hyprwm/Hyprland/blob/1ac1ff457ab8ef1ae6a8f2ab17ee7965adfa729f/src/managers/KeybindManager.cpp#L987
pub fn spawn_sh(command: String, token: Option<XdgActivationToken>) {
    spawn(vec![String::from("sh"), String::from("-c"), command], token);
}

fn spawn_sync(
    command: impl AsRef<OsStr>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    token: Option<XdgActivationToken>,
) {
    let _span = tracy_client::span!();

    let mut command = command.as_ref();

    // Expand `~` at the start.
    let expanded = expand_home(Path::new(command));
    match &expanded {
        Ok(Some(expanded)) => command = expanded.as_ref(),
        Ok(None) => (),
        Err(err) => {
            warn!("error expanding ~: {err:?}");
        }
    }

    let mut request = SpawnRequest {
        program: command.to_owned(),
        args: args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect(),
        environment: Vec::new(),
        nofile_soft: ORIGINAL_NOFILE_RLIMIT_CUR.load(Ordering::SeqCst),
        nofile_hard: ORIGINAL_NOFILE_RLIMIT_MAX.load(Ordering::SeqCst),
        systemd_scope: IS_SYSTEMD_SERVICE.load(Ordering::Relaxed),
    };

    // Remove RUST_BACKTRACE and RUST_LIB_BACKTRACE from the environment if needed.
    if REMOVE_ENV_RUST_BACKTRACE.load(Ordering::Relaxed) {
        request.environment.push(("RUST_BACKTRACE".into(), None));
    }
    if REMOVE_ENV_RUST_LIB_BACKTRACE.load(Ordering::Relaxed) {
        request
            .environment
            .push(("RUST_LIB_BACKTRACE".into(), None));
    }

    // Remove the systemd NOTIFY_SOCKET variable.
    request.environment.push(("NOTIFY_SOCKET".into(), None));

    // Set DISPLAY if needed.
    let display = CHILD_DISPLAY.read().unwrap();
    request
        .environment
        .push(("DISPLAY".into(), display.as_ref().map(OsString::from)));
    drop(display);

    // Set configured environment.
    let env = CHILD_ENV.read().unwrap();
    request.environment.extend(env.0.iter().map(|var| {
        (
            OsString::from(&var.name),
            var.value.as_ref().map(OsString::from),
        )
    }));
    drop(env);

    if let Some(token) = token.as_ref() {
        request
            .environment
            .push(("XDG_ACTIVATION_TOKEN".into(), Some(token.as_str().into())));
        request
            .environment
            .push(("DESKTOP_STARTUP_ID".into(), Some(token.as_str().into())));
    }

    let mut child = match spawn_via_helper(&request) {
        Ok(child) => child,
        Err(err) => {
            warn!("error spawning helper for {command:?}: {err:?}");
            return;
        }
    };

    match child.wait() {
        Ok(status) => {
            if !status.success() {
                warn!("child did not exit successfully: {status:?}");
            }
        }
        Err(err) => {
            warn!("error waiting for child: {err:?}");
        }
    }
}

#[cfg(feature = "systemd")]
mod systemd {
    use super::*;

    /// Puts a (newly spawned) pid into a transient systemd scope.
    ///
    /// This separates the pid from the compositor scope, which for example prevents the OOM killer
    /// from bringing down the compositor together with a misbehaving client.
    pub(super) fn start_systemd_scope(
        name: &OsStr,
        intermediate_pid: u32,
        child_pid: u32,
    ) -> anyhow::Result<()> {
        use std::fmt::Write as _;
        use std::os::unix::ffi::OsStrExt;
        use std::sync::OnceLock;

        use anyhow::Context;
        use zbus::zvariant::{OwnedObjectPath, Value};

        use crate::utils::IS_SYSTEMD_SERVICE;

        // We only start transient scopes if we're a systemd service ourselves.
        if !IS_SYSTEMD_SERVICE.load(Ordering::Relaxed) {
            return Ok(());
        }

        let _span = tracy_client::span!();

        // Extract the basename.
        let name = Path::new(name).file_name().unwrap_or(name);

        let mut scope_name = String::from("app-niri-");

        // Escape for systemd similarly to libgnome-desktop, which says it had adapted this from
        // systemd source.
        for &c in name.as_bytes() {
            if c.is_ascii_alphanumeric() || matches!(c, b':' | b'_' | b'.') {
                scope_name.push(char::from(c));
            } else {
                let _ = write!(scope_name, "\\x{c:02x}");
            }
        }

        let _ = write!(scope_name, "-{child_pid}.scope");

        // Ask systemd to start a transient scope.
        static CONNECTION: OnceLock<zbus::Result<zbus::blocking::Connection>> = OnceLock::new();
        let conn = CONNECTION
            .get_or_init(zbus::blocking::Connection::session)
            .clone()
            .context("error connecting to session bus")?;

        let proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .context("error creating a Proxy")?;

        let signals = proxy
            .receive_signal("JobRemoved")
            .context("error creating a signal iterator")?;

        let pids: &[_] = &[intermediate_pid, child_pid];
        let properties: &[_] = &[
            ("PIDs", Value::new(pids)),
            ("CollectMode", Value::new("inactive-or-failed")),
        ];
        let aux: &[(&str, &[(&str, Value)])] = &[];

        let job: OwnedObjectPath = proxy
            .call("StartTransientUnit", &(scope_name, "fail", properties, aux))
            .context("error calling StartTransientUnit")?;

        trace!("waiting for JobRemoved");
        for message in signals {
            let body = message.body();
            let body: (u32, OwnedObjectPath, &str, &str) =
                body.deserialize().context("error parsing signal")?;

            if body.1 == job {
                // Our transient unit had started, we're good to exit the intermediate child.
                break;
            }
        }

        Ok(())
    }
}
