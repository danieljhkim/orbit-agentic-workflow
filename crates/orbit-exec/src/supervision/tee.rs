use std::io::{Read, Write};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};

use orbit_types::redact_sensitive_env_text;

pub(super) fn spawn_stdout_drain<R>(mut out: R, debug: bool) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        if debug {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                match out.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        // Redact sensitive env values before printing to stderr
                        // so tokens/secrets are never shown in debug output.
                        let raw = String::from_utf8_lossy(&chunk[..n]);
                        let redacted = redact_sensitive_env_text(&raw);
                        let _ = std::io::stderr().write_all(redacted.as_bytes());
                        buf.extend_from_slice(&chunk[..n]);
                    }
                }
            }
            buf
        } else {
            let mut buf = Vec::new();
            let _ = out.read_to_end(&mut buf);
            buf
        }
    })
}

pub(super) fn spawn_stderr_drain<R>(mut err: R, debug: bool) -> JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        if debug {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                match err.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let redacted = redact_chunk(&chunk[..n]);
                        let _ = std::io::stderr().write_all(&redacted);
                        buf.extend_from_slice(&redacted);
                    }
                }
            }
            buf
        } else {
            let mut buf = Vec::new();
            let _ = err.read_to_end(&mut buf);
            buf
        }
    })
}

pub(super) fn spawn_stdin_write<W>(
    mut stdin: W,
    bytes: Vec<u8>,
    result_tx: Sender<Result<(), String>>,
) -> JoinHandle<()>
where
    W: Write + Send + 'static,
{
    thread::spawn(move || {
        let result = stdin
            .write_all(&bytes)
            .map_err(|e| format!("failed to write process stdin: {e}"));
        let _ = result_tx.send(result);
    })
}

fn redact_chunk(chunk: &[u8]) -> Vec<u8> {
    redact_sensitive_env_text(&String::from_utf8_lossy(chunk)).into_bytes()
}
