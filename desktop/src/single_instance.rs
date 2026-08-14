// Watermelon Vector Converter — Desktop
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
//
// Native single-instance detection, replacing tauri-plugin-single-instance
// (which no longer applies now that Tauri is gone). Uses a fixed-port TCP
// loopback listener: whichever process binds the port first "owns" the
// instance; every later launch that finds the port already taken sends the
// new file path to the owner and exits.
//
// A TCP loopback socket (rather than e.g. a lock file + signal) was chosen
// because it needs no OS-specific IPC mechanism at all — works identically
// on Windows/macOS/Linux with only std, and doubles as the exact mechanism
// for "tell the running instance to open this file", not just detection.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

/// Loopback-only, fixed port. High/uncommon enough to avoid colliding with
/// other local dev tools; loopback-bound so nothing outside this machine
/// can ever reach it.
const SINGLE_INSTANCE_PORT: u16 = 47821;

/// If another instance is already listening, send it `path` and return
/// true (caller should exit immediately without opening a window). If we
/// successfully become the owning instance, spawns a background thread to
/// keep listening for further file-open requests and returns false.
pub fn notify_existing_instance(path: &Path) -> bool {
    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", SINGLE_INSTANCE_PORT)) {
        let path_str = path.to_string_lossy();
        let _ = stream.write_all(path_str.as_bytes());
        let _ = stream.flush();
        return true;
    }
    false
}

/// Starts listening for other-instance file-open requests. Returns a
/// Receiver the iced application subscribes to (via a Task/Subscription)
/// so an incoming path can update the already-open viewer window instead
/// of a new process ever needing to exist.
pub fn start_listener() -> Option<Receiver<PathBuf>> {
    let listener = TcpListener::bind(("127.0.0.1", SINGLE_INSTANCE_PORT)).ok()?;
    let (tx, rx): (Sender<PathBuf>, Receiver<PathBuf>) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut buf = Vec::new();
            if stream.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                if let Ok(s) = String::from_utf8(buf) {
                    let _ = tx.send(PathBuf::from(s));
                }
            }
        }
    });

    Some(rx)
}
