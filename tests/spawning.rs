//! A separate process is necessary for signal masks, rlimits and pthread_atfork.
//! Unlike a timing assertion, the atfork hook detects the compositor-side fork
//! deterministically, without allocating gigabytes or depending on machine load.

use std::ffi::OsString;
use std::io::Write as _;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{env, io};

use niri::utils::spawning::{
    spawn, spawn_sh, store_and_increase_nofile_rlimit, CHILD_DISPLAY, CHILD_ENV,
    REMOVE_ENV_RUST_BACKTRACE, REMOVE_ENV_RUST_LIB_BACKTRACE,
};
use niri_config::Config;
use serde_json::{json, Value};

static FORKS: AtomicUsize = AtomicUsize::new(0);

extern "C" fn before_fork() {
    FORKS.fetch_add(1, Ordering::Relaxed);
}

struct Reports {
    path: PathBuf,
    socket: UnixDatagram,
}

impl Reports {
    fn new() -> io::Result<Self> {
        let path = env::temp_dir().join(format!(
            "niri-spawning-{}-{}.sock",
            std::process::id(),
            fastrand::u64(..)
        ));
        let socket = UnixDatagram::bind(&path)?;
        socket.set_read_timeout(Some(Duration::from_secs(10)))?;
        Ok(Self { path, socket })
    }

    fn receive(&self) -> Value {
        let mut buf = [0; 65536];
        let len = self
            .socket
            .recv(&mut buf)
            .expect("spawned application report");
        serde_json::from_slice(&buf[..len]).unwrap()
    }
}

impl Drop for Reports {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn nofile() -> libc::rlimit {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) },
        0
    );
    limit
}

fn probe(mut args: impl Iterator<Item = OsString>) -> anyhow::Result<()> {
    let path = args.next().unwrap();
    let args: Vec<_> = args.map(|arg| arg.as_bytes().to_vec()).collect();
    let limit = nofile();
    let mut mask = std::mem::MaybeUninit::<libc::sigset_t>::uninit();
    assert_eq!(
        unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, std::ptr::null(), mask.as_mut_ptr()) },
        0
    );
    let mask = unsafe { mask.assume_init() };
    let blocked: Vec<_> = [libc::SIGINT, libc::SIGTERM, libc::SIGHUP]
        .map(|signal| unsafe { libc::sigismember(&mask, signal) })
        .into();
    let report = json!({
        "args": args,
        "configured": env::var("NIRI_SPAWN_TEST_VALUE").ok(),
        "large_env": env::var("NIRI_SPAWN_TEST_LARGE").ok(),
        "removed": env::var_os("NIRI_SPAWN_TEST_REMOVE").is_none(),
        "raw_env": env::var_os("NIRI_SPAWN_TEST_RAW").map(OsString::into_vec),
        "display": env::var("DISPLAY").ok(),
        "activation": env::var("XDG_ACTIVATION_TOKEN").ok(),
        "startup": env::var("DESKTOP_STARTUP_ID").ok(),
        "backtrace": env::var("RUST_BACKTRACE").ok(),
        "lib_backtrace": env::var_os("RUST_LIB_BACKTRACE").is_some(),
        "notify_socket": env::var_os("NOTIFY_SOCKET").is_some(),
        "soft_limit": limit.rlim_cur,
        "hard_limit": limit.rlim_max,
        "blocked": blocked,
    });
    UnixDatagram::unbound()?.send_to(&serde_json::to_vec(&report)?, path)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    if let Some(result) = niri::utils::spawning::run_spawn_helper() {
        return result;
    }
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() == Some(std::ffi::OsStr::new("--probe")) {
        return probe(args);
    }

    let reports = Reports::new()?;
    let original_limit = nofile();
    store_and_increase_nofile_rlimit();
    niri::utils::signals::block_early()?;
    env::set_var("NIRI_SPAWN_TEST_REMOVE", "inherited");
    env::set_var("NIRI_SPAWN_TEST_RAW", OsString::from_vec(vec![0xfe, b'x']));
    env::set_var("RUST_BACKTRACE", "1");
    env::set_var("RUST_LIB_BACKTRACE", "0");
    env::set_var("NOTIFY_SOCKET", "/not-a-notify-socket");
    REMOVE_ENV_RUST_BACKTRACE.store(true, Ordering::Relaxed);
    REMOVE_ENV_RUST_LIB_BACKTRACE.store(true, Ordering::Relaxed);
    *CHILD_DISPLAY.write().unwrap() = Some(":987".to_owned());
    *CHILD_ENV.write().unwrap() = Config::parse_mem(
        r#"
        environment {
            NIRI_SPAWN_TEST_VALUE "spaces ' quotes --env"
            NIRI_SPAWN_TEST_REMOVE null
            RUST_BACKTRACE "configured"
        }
    "#,
    )?
    .environment;
    // OsString's JSON byte representation exceeds pipe capacity; startup must drain it
    // concurrently rather than waiting for EOF before the parent can finish writing.
    CHILD_ENV
        .write()
        .unwrap()
        .0
        .push(niri_config::EnvironmentVariable {
            name: "NIRI_SPAWN_TEST_LARGE".to_owned(),
            value: Some("x".repeat(32768)),
        });
    assert_eq!(
        unsafe { libc::pthread_atfork(Some(before_fork), None, None) },
        0
    );

    let executable = env::current_exe()?;
    let raw_arg = OsString::from_vec(vec![b'a', 0xff, b'b']);
    spawn(
        vec![
            executable.clone().into_os_string(),
            "--probe".into(),
            reports.path.clone().into_os_string(),
            raw_arg.clone(),
            "".into(),
            "--env".into(),
            "a 'quoted' argument".into(),
        ],
        Some("spawn-test-token".to_owned().into()),
    );
    let report = reports.receive();
    assert_eq!(
        report["args"],
        json!([
            raw_arg.as_bytes(),
            b"".as_slice(),
            b"--env".as_slice(),
            b"a 'quoted' argument".as_slice(),
        ])
    );
    assert_eq!(report["configured"], "spaces ' quotes --env");
    assert_eq!(report["large_env"], "x".repeat(32768));
    assert_eq!(report["removed"], true);
    assert_eq!(report["raw_env"], json!([254, 120]));
    assert_eq!(report["display"], ":987");
    assert_eq!(report["activation"], "spawn-test-token");
    assert_eq!(report["startup"], "spawn-test-token");
    assert_eq!(report["backtrace"], "configured");
    assert_eq!(report["lib_backtrace"], false);
    assert_eq!(report["notify_socket"], false);
    assert_eq!(report["soft_limit"], original_limit.rlim_cur);
    assert_eq!(report["hard_limit"], original_limit.rlim_max);
    assert_eq!(report["blocked"], json!([0, 0, 0]));
    if cfg!(any(target_os = "linux", target_os = "freebsd")) {
        assert_eq!(
            FORKS.load(Ordering::Relaxed),
            0,
            "application spawning forked the compositor"
        );
    }

    let quote =
        |path: &std::path::Path| format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"));
    spawn_sh(
        format!(
            "exec {} --probe {} \"$NIRI_SPAWN_TEST_VALUE\"",
            quote(&executable),
            quote(&reports.path)
        ),
        None,
    );
    let report = reports.receive();
    assert_eq!(report["args"], json!([b"spaces ' quotes --env".as_slice()]));
    if cfg!(any(target_os = "linux", target_os = "freebsd")) {
        assert_eq!(
            FORKS.load(Ordering::Relaxed),
            0,
            "shell spawning forked the compositor"
        );
    }

    // Failed exec used to deadlock the intermediate-child handshake. Check both missing
    // and non-executable programs, and an incomplete transport request, with bounded cleanup.
    let mut requests = Vec::new();
    for program in [reports.path.with_extension("missing"), reports.path.clone()] {
        requests.push(serde_json::to_vec(&json!({
            "program": program.into_os_string(),
            "args": [],
            "environment": [],
            "nofile_soft": original_limit.rlim_cur,
            "nofile_hard": original_limit.rlim_max,
            "systemd_scope": false,
        }))?);
    }
    requests.push(b"{".to_vec());
    for request in requests {
        let mut child = Command::new(&executable)
            .arg("--internal-spawn")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        child.stdin.take().unwrap().write_all(&request)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                panic!("failed application launch left a stuck helper");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(
            !status.success(),
            "invalid launch was reported as successful"
        );
    }
    println!("spawn process contracts passed");
    Ok(())
}
