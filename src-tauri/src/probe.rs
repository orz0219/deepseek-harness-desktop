use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dsh_launcher::parse_status_code;

use crate::{KILL_GRACE, POLL_INTERVAL, READY_TIMEOUT};

pub fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
}

/// Minimal HTTP/1.0 GET; returns the response status code, or `None` when the
/// server did not answer in time.
pub fn http_probe(host: &str, port: u16, path: &str) -> Option<u16> {
    let addr: SocketAddr = {
        let mut addrs: Vec<_> = (host, port).to_socket_addrs().ok()?.collect();
        addrs.pop()?
    };
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    let req = format!(
        "GET {path} HTTP/1.0\r\nHost: {host}:{port}\r\nConnection: close\r\nAccept: */*\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).ok()?;
    parse_status_code(&String::from_utf8_lossy(&buf[..n]))
}

/// Signature test for "a dsh web instance is serving on this port":
///
/// * `GET /` and `GET /manifest.webmanifest` → 200 (the agreed readiness signal,
///   matching `@linxin666/dsh-desktop-launcher`), **and**
/// * `GET /api` → 404 (dsh mounts the RPC namespace; unknown SPA paths return
///   the catch-all HTML 200 — FEASIBILITY 门禁 0).
///
/// The third probe filters out unrelated local servers that happen to answer
/// 200 on both static paths. Residual ambiguity (another dsh-shaped service)
/// is accepted per the "follow dsh's current behavior" principle.
pub fn dsh_signature_ok(port: u16) -> bool {
    http_probe("127.0.0.1", port, "/") == Some(200)
        && http_probe("127.0.0.1", port, "/manifest.webmanifest") == Some(200)
        && http_probe("127.0.0.1", port, "/api") == Some(404)
}

/// Poll the readiness signal until satisfied or the timeout elapses.
pub fn wait_for_ready(port: u16) -> bool {
    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        if dsh_signature_ok(port) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Spawn dsh in its own process group (so we can tear down the whole tree) and
/// return the child plus its continuously-drained stderr capture (PLAN Slice 1B).
pub fn spawn_dsh(plan: &dsh_launcher::LaunchPlan) -> std::io::Result<(Child, StderrCapture)> {
    let mut cmd = Command::new(&plan.program);
    cmd.args(&plan.args)
        .envs(&plan.env)
        .current_dir(&plan.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // New process group: pgid == pid, so `kill -- -pid` kills dsh + workers.
    cmd.process_group(0);
    let mut child = cmd.spawn()?;
    let stderr = child.stderr.take().expect("stderr was piped");
    let capture = StderrCapture::attach(stderr);
    Ok((child, capture))
}

/// Send SIGTERM to the whole process group, poll up to KILL_GRACE, then
/// SIGKILL any survivors. Returns whether the group is confirmed gone (PLAN
/// 验收点 4: ps tree verification).
///
/// Polling (instead of an unconditional sleep) keeps app quit instant when dsh
/// goes down promptly.
pub fn terminate_tree(pgid: i32) -> bool {
    let _ = Command::new("kill")
        .args(["-TERM", "--", &format!("-{pgid}")])
        .output();
    let deadline = Instant::now() + KILL_GRACE;
    while Instant::now() < deadline {
        if !pgid_alive(pgid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .output();
    let kill_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < kill_deadline {
        if !pgid_alive(pgid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    !pgid_alive(pgid)
}

/// Heuristic: is any process still in the group? Use `pgrep -g <pgid>`.
fn pgid_alive(pgid: i32) -> bool {
    match Command::new("pgrep")
        .arg("-g")
        .arg(pgid.to_string())
        .output()
    {
        Ok(out) => !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        Err(_) => false,
    }
}

/// 当 launcher 未拥有 dsh（外部实例，pgid 为 None）时，按端口找到占用进程
/// 并发送 TERM，返回被终止的 pid 列表，便于随后做 KILL 兜底。
/// lsof 不在默认 PATH 中，使用绝对路径。
pub fn kill_port_owner(port: u16) -> Vec<i32> {
    let out = match Command::new("/usr/sbin/lsof")
        .args(["-ti", &format!("tcp:{port}")])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    let mut pids = Vec::new();
    for tok in String::from_utf8_lossy(&out.stdout).split_whitespace() {
        if let Ok(pid) = tok.parse::<i32>() {
            let _ = Command::new("kill")
                .args(["-TERM", "--", &pid.to_string()])
                .status();
            pids.push(pid);
        }
    }
    pids
}

/// Continuously drained stderr ring buffer (last ~16 KiB).
///
/// Spawning with `Stdio::piped()` and only reading after exit deadlocks the
/// child once it fills the OS pipe buffer (~64 KiB) — dsh freezes mid-write
/// and looks hung. A dedicated reader thread keeps the pipe empty at all times
/// and retains a bounded tail for post-mortem diagnostics.
pub struct StderrCapture {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl StderrCapture {
    const CAPACITY: usize = 16 * 1024;

    pub fn attach(mut stderr: ChildStderr) -> Self {
        let buf = Arc::new(Mutex::new(Vec::with_capacity(Self::CAPACITY)));
        let writer = buf.clone();
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            loop {
                match stderr.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut b = writer.lock().unwrap();
                        b.extend_from_slice(&chunk[..n]);
                        if b.len() > Self::CAPACITY {
                            let drop = b.len() - Self::CAPACITY;
                            b.drain(..drop);
                        }
                    }
                }
            }
        });
        StderrCapture { buf }
    }

    pub fn tail(&self) -> String {
        let b = self.buf.lock().unwrap();
        String::from_utf8_lossy(&b[..]).trim().to_string()
    }
}
