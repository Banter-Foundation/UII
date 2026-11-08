//! Loopback plug the native app serves, and the file Cursor reads when the app is closed.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::model::{LayoutError, LayoutSnapshot};
use crate::paths::default_snapshot_path;

const MAX_MESSAGE: u64 = 16 * 1024 * 1024;

/// Where to read the current layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOptions {
    pub plug_addr: Option<SocketAddr>,
    pub snapshot_path: Option<PathBuf>,
}

/// Live snapshot server. Bind it from the desktop app and [`LayoutPlug::publish`] on each edit.
///
/// Listens on loopback only. One JSON line in, one JSON line out: `{"method":"layout"}`.
pub struct LayoutPlug {
    addr: SocketAddr,
    state: Arc<RwLock<Option<LayoutSnapshot>>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for LayoutPlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutPlug")
            .field("addr", &self.addr)
            .finish_non_exhaustive()
    }
}

impl LayoutPlug {
    pub fn bind() -> Result<Self, LayoutError> {
        Self::bind_addr(SocketAddr::from(([127, 0, 0, 1], 0)))
    }

    pub fn bind_addr(addr: SocketAddr) -> Result<Self, LayoutError> {
        if !addr.ip().is_loopback() {
            return Err(LayoutError::Plug("plug listens on loopback only".into()));
        }
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let local = listener.local_addr()?;
        let state = Arc::new(RwLock::new(None));
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_state = Arc::clone(&state);
        let thread_shutdown = Arc::clone(&shutdown);
        let thread = thread::spawn(move || serve(listener, thread_state, thread_shutdown));
        Ok(Self {
            addr: local,
            state,
            shutdown,
            thread: Some(thread),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn publish(&self, snapshot: LayoutSnapshot) -> Result<(), LayoutError> {
        snapshot.validate()?;
        let mut guard = self.state.write().unwrap_or_else(|err| err.into_inner());
        *guard = Some(snapshot);
        Ok(())
    }

    /// Publish in memory and write `layout.json` plus `plug.addr` beside it.
    pub fn publish_to(
        &self,
        snapshot: LayoutSnapshot,
        snapshot_path: &Path,
    ) -> Result<(), LayoutError> {
        snapshot.validate()?;
        write_snapshot(snapshot_path, &snapshot)?;
        let addr_path = snapshot_path.with_file_name("plug.addr");
        atomic_write(&addr_path, format!("{}\n", self.addr).as_bytes())?;
        self.publish(snapshot)
    }
}

impl Drop for LayoutPlug {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

fn serve(
    listener: TcpListener,
    state: Arc<RwLock<Option<LayoutSnapshot>>>,
    shutdown: Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, peer)) => {
                if peer.ip().is_loopback() {
                    let _ = handle_client(stream, &state);
                }
            }
            Err(err) if is_retry(err.kind()) => thread::sleep(Duration::from_millis(20)),
            Err(_) => break,
        }
    }
}

fn is_retry(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
    )
}

fn handle_client(
    mut stream: TcpStream,
    state: &RwLock<Option<LayoutSnapshot>>,
) -> Result<(), LayoutError> {
    stream.set_nonblocking(false)?;
    let _ = stream.set_nodelay(true);
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = Vec::new();
    let n = reader
        .by_ref()
        .take(MAX_MESSAGE)
        .read_until(b'\n', &mut line)?;
    if n == 0 {
        return Ok(());
    }
    trim_newline(&mut line);
    let response = match reply_line(&line, state) {
        Ok(body) => body,
        Err(err) => serde_json::json!({ "error": err.to_string() }).to_string(),
    };
    stream.write_all(response.as_bytes())?;
    stream.write_all(b"\n")?;
    Ok(())
}

fn reply_line(line: &[u8], state: &RwLock<Option<LayoutSnapshot>>) -> Result<String, LayoutError> {
    let value: serde_json::Value = serde_json::from_slice(line)?;
    let method = value.get("method").and_then(|m| m.as_str()).unwrap_or("");
    if method != "layout" {
        return Err(LayoutError::Plug("unknown method".into()));
    }
    let guard = state.read().unwrap_or_else(|err| err.into_inner());
    match guard.as_ref() {
        Some(snapshot) => Ok(serde_json::to_string(snapshot)?),
        None => Err(LayoutError::NoLayout),
    }
}

fn trim_newline(line: &mut Vec<u8>) {
    if line.last() == Some(&b'\n') {
        line.pop();
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
}

pub fn fetch_live(addr: SocketAddr) -> Result<LayoutSnapshot, LayoutError> {
    if !addr.ip().is_loopback() {
        return Err(LayoutError::Plug("plug listens on loopback only".into()));
    }
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))
        .map_err(|_| LayoutError::PlugUnreachable)?;
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .map_err(|_| LayoutError::PlugUnreachable)?;
    stream
        .set_write_timeout(Some(Duration::from_millis(500)))
        .map_err(|_| LayoutError::PlugUnreachable)?;
    stream
        .write_all(b"{\"method\":\"layout\"}\n")
        .map_err(|_| LayoutError::PlugUnreachable)?;
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    let n = reader
        .by_ref()
        .take(MAX_MESSAGE)
        .read_until(b'\n', &mut line)
        .map_err(|_| LayoutError::PlugUnreachable)?;
    if n == 0 {
        return Err(LayoutError::PlugUnreachable);
    }
    trim_newline(&mut line);
    let value: serde_json::Value = serde_json::from_slice(&line)?;
    if let Some(err) = value.get("error").and_then(|err| err.as_str()) {
        return Err(LayoutError::Plug(err.to_string()));
    }
    let snapshot: LayoutSnapshot = serde_json::from_value(value)?;
    snapshot.validate()?;
    Ok(snapshot)
}

pub fn read_current(opts: &ReadOptions) -> Result<LayoutSnapshot, LayoutError> {
    if let Some(addr) = opts.plug_addr {
        match fetch_live(addr) {
            Ok(snapshot) => return Ok(snapshot),
            Err(LayoutError::PlugUnreachable) => {}
            Err(err) => return Err(err),
        }
    }
    if let Some(path) = &opts.snapshot_path {
        if path.is_file() {
            return read_snapshot(path);
        }
    }
    Err(LayoutError::NoLayout)
}

pub fn discover(snapshot_path: &Path) -> ReadOptions {
    let plug_addr = read_optional_addr(&snapshot_path.with_file_name("plug.addr"));
    ReadOptions {
        plug_addr,
        snapshot_path: Some(snapshot_path.to_path_buf()),
    }
}

pub fn options_from_env() -> Result<ReadOptions, LayoutError> {
    let snapshot_path = match std::env::var("UII_LAYOUT_SNAPSHOT") {
        Ok(value) => PathBuf::from(value),
        Err(_) => default_snapshot_path(),
    };
    let plug_addr = match std::env::var("UII_PLUG_ADDR") {
        Ok(value) => Some(parse_addr(&value)?),
        Err(_) => read_optional_addr(&snapshot_path.with_file_name("plug.addr")),
    };
    Ok(ReadOptions {
        plug_addr,
        snapshot_path: Some(snapshot_path),
    })
}

fn parse_addr(value: &str) -> Result<SocketAddr, LayoutError> {
    value
        .trim()
        .parse()
        .map_err(|_| LayoutError::Plug(format!("invalid plug address: {value}")))
}

fn read_optional_addr(path: &Path) -> Option<SocketAddr> {
    let text = fs::read_to_string(path).ok()?;
    text.trim().parse().ok()
}

pub fn write_snapshot(path: &Path, snapshot: &LayoutSnapshot) -> Result<(), LayoutError> {
    snapshot.validate()?;
    let mut bytes = serde_json::to_vec_pretty(snapshot)?;
    bytes.push(b'\n');
    atomic_write(path, &bytes)
}

pub fn read_snapshot(path: &Path) -> Result<LayoutSnapshot, LayoutError> {
    let text = fs::read_to_string(path)?;
    let snapshot: LayoutSnapshot = serde_json::from_str(&text)?;
    snapshot.validate()?;
    Ok(snapshot)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), LayoutError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "layout.json".to_string());
    let tmp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    fs::write(&tmp, bytes)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Windows rename does not replace an existing file.
            let _ = fs::remove_file(path);
            let renamed = fs::rename(&tmp, path);
            if renamed.is_err() {
                let _ = fs::remove_file(&tmp);
            }
            renamed.map_err(LayoutError::from)
        }
    }
}
