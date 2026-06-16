use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::io;
use std::os::unix::io::RawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Maximum wait between `try_send` attempts when the channel is full.
const SUBMIT_POLL_TIMEOUT: Duration = Duration::from_millis(2);
/// How long `shutdown` waits for the writer to finish before giving up.
const SHUTDOWN_JOIN_TIMEOUT: Duration = Duration::from_millis(1000);

/// Outcome of a chunked write.
pub enum WriteOutcome {
    /// The entire buffer was written.
    Complete,
    /// A quit was signalled mid-write (only part of the buffer was written).
    QuitSignaled,
}

/// Write `buf` to `fd` in 16KB chunks, calling `should_quit` before each chunk
/// and after each successful chunk. Returns `QuitSignaled` if `should_quit`
/// returned `Ok(true)` before the buffer was fully written.
///
/// Shared by the inline (main-thread) write path and the threaded writer so the
/// on-wire bytes stay identical between `--single-threaded` and threaded mode.
pub fn write_chunked(
    fd: RawFd,
    buf: &[u8],
    mut should_quit: impl FnMut() -> io::Result<bool>,
) -> io::Result<WriteOutcome> {
    let mut written = 0;
    while written < buf.len() {
        if should_quit()? {
            return Ok(WriteOutcome::QuitSignaled);
        }
        let chunk_end = (written + 16384).min(buf.len());
        let n = unsafe {
            libc::write(
                fd,
                buf[written..chunk_end].as_ptr() as *const libc::c_void,
                chunk_end - written,
            )
        };
        if n > 0 {
            written += n as usize;
            if should_quit()? {
                return Ok(WriteOutcome::QuitSignaled);
            }
        } else if n < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }
    }
    Ok(WriteOutcome::Complete)
}

/// Returns true if the given key event is a quit gesture (a configured quit key
/// or Ctrl+C). Shared by the inline write closure and the threaded submit loop.
#[allow(dead_code)] // wired in Task 3
pub fn is_quit_key(code: KeyCode, modifiers: KeyModifiers, quit_keys: &[KeyCode]) -> bool {
    quit_keys.contains(&code)
        || (code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
}

/// Result of submitting a frame to a `ThreadedRenderer`.
#[allow(dead_code)] // wired in Task 3
pub enum SubmitResult {
    /// Frame was accepted (sent to the writer thread).
    Ok,
    /// A quit was observed while submitting.
    Quit,
    /// The writer thread has exited (channel disconnected).
    WriterDied,
}

/// Frame sink that writes on a dedicated thread. The main loop hands off owned
/// `Vec<u8>` frame buffers via a bounded channel; the writer does the chunked
/// `libc::write()` and publishes its measured write time.
#[allow(dead_code)] // wired in Task 3
pub struct ThreadedRenderer {
    tx: Option<SyncSender<Vec<u8>>>,
    write_time: Arc<AtomicU64>,
    handle: Option<JoinHandle<()>>,
}

#[allow(dead_code)] // wired in Task 3
impl ThreadedRenderer {
    /// Spawn the writer thread. The thread holds a clone of `quit` so it can bail
    /// between write chunks; `fd` is the raw terminal fd (stdout, fd 1).
    pub fn new(quit: Arc<AtomicBool>, fd: RawFd) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(1);
        let write_time = Arc::new(AtomicU64::new(0));
        let write_time_w = write_time.clone();
        let quit_w = quit;
        let handle = thread::spawn(move || {
            while let Ok(buf) = rx.recv() {
                let start = Instant::now();
                match write_chunked(fd, &buf, || Ok(quit_w.load(Ordering::Relaxed))) {
                    Ok(_) => {
                        write_time_w.store(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
                    }
                    Err(_) => break, // write failed → exit so the main loop notices via WriterDied
                }
            }
        });
        ThreadedRenderer {
            tx: Some(tx),
            write_time,
            handle: Some(handle),
        }
    }

    /// Hand a fully-built frame buffer to the writer, staying responsive to quit
    /// while the channel is full. `quit` is set when a quit key is observed.
    pub fn submit(
        &mut self,
        mut buf: Vec<u8>,
        quit: &AtomicBool,
        quit_keys: &[KeyCode],
    ) -> io::Result<SubmitResult> {
        let tx = match &self.tx {
            Some(tx) => tx,
            None => return Ok(SubmitResult::WriterDied),
        };
        loop {
            if quit.load(Ordering::Relaxed) {
                return Ok(SubmitResult::Quit);
            }
            match tx.try_send(buf) {
                Ok(()) => return Ok(SubmitResult::Ok),
                Err(mpsc::TrySendError::Full(b)) => {
                    buf = b;
                    if event::poll(SUBMIT_POLL_TIMEOUT)? && poll_quit(quit, quit_keys)? {
                        return Ok(SubmitResult::Quit);
                    }
                }
                Err(mpsc::TrySendError::Disconnected(_)) => return Ok(SubmitResult::WriterDied),
            }
        }
    }

    /// Wall-clock seconds of the most recent completed write (0.0 until the first
    /// frame finishes writing).
    pub fn write_time_secs(&self) -> f64 {
        self.write_time.load(Ordering::Relaxed) as f64 / 1e9
    }

    /// Close the channel and wait (bounded) for the writer to exit. Must be
    /// called before the main thread writes terminal-restore bytes, so the two
    /// never interleave on the fd.
    pub fn shutdown(mut self) -> io::Result<()> {
        // Drop the sender first so the writer's recv() returns Err and the thread exits.
        self.tx.take();
        if let Some(handle) = self.handle.take() {
            // std has no join_timeout; use a helper thread + recv_timeout.
            let (done_tx, done_rx) = mpsc::channel();
            thread::spawn(move || {
                let _ = handle.join();
                let _ = done_tx.send(());
            });
            let _ = done_rx.recv_timeout(SHUTDOWN_JOIN_TIMEOUT);
        }
        Ok(())
    }
}

/// Read one pending input event (event::poll already confirmed one is ready) and
/// return true if it is a quit gesture, setting `quit` when it is.
#[allow(dead_code)] // wired in Task 3 (called by ThreadedRenderer::submit)
fn poll_quit(quit: &AtomicBool, quit_keys: &[KeyCode]) -> io::Result<bool> {
    if let Event::Key(KeyEvent {
        code,
        kind: KeyEventKind::Press,
        modifiers,
        ..
    }) = event::read()?
        && is_quit_key(code, modifiers, quit_keys)
    {
        quit.store(true, Ordering::Release);
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;
    use std::io::Read;
    use std::os::unix::io::{AsRawFd, FromRawFd};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};

    fn make_pipe() -> (std::fs::File, std::fs::File) {
        let mut fds = [0i32; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
        let read_end = unsafe { std::fs::File::from_raw_fd(fds[0]) };
        let write_end = unsafe { std::fs::File::from_raw_fd(fds[1]) };
        (read_end, write_end)
    }

    #[test]
    fn write_chunked_writes_all_bytes_when_no_quit() {
        let (mut read_end, write_end) = make_pipe();
        let fd = write_end.as_raw_fd();
        let data = b"hello threaded world".repeat(100); // ~2KB, well under the pipe buffer
        let outcome = write_chunked(fd, &data, || Ok(false)).unwrap();
        assert!(matches!(outcome, WriteOutcome::Complete));
        drop(write_end); // close write end so the reader sees EOF
        let mut got = Vec::new();
        read_end.read_to_end(&mut got).unwrap();
        assert_eq!(got, data);
    }

    #[test]
    fn write_chunked_bails_when_quit_signalled() {
        let (mut read_end, write_end) = make_pipe();
        let fd = write_end.as_raw_fd();
        // Larger than one 16KB chunk so the quit check fires mid-stream.
        let data = vec![b'x'; 40_000];
        let mut calls = 0;
        let outcome = write_chunked(fd, &data, || {
            calls += 1;
            Ok(calls >= 2) // quit on the 2nd check (after first chunk written)
        })
        .unwrap();
        assert!(matches!(outcome, WriteOutcome::QuitSignaled));
        drop(write_end);
        let mut got = Vec::new();
        read_end.read_to_end(&mut got).unwrap();
        assert!(!got.is_empty(), "expected at least one chunk written");
        assert!(
            got.len() < data.len(),
            "wrote {} bytes, expected a partial write",
            got.len()
        );
    }

    #[test]
    fn threaded_renderer_writes_submitted_frame() {
        let (read_end, write_end) = make_pipe();
        let fd = write_end.as_raw_fd();
        let quit = Arc::new(AtomicBool::new(false));
        let mut renderer = ThreadedRenderer::new(quit.clone(), fd);
        let frame = b"\x1b[?2026hframe1\x1b[?2026l".to_vec();
        let res = renderer
            .submit(frame, &quit, &[KeyCode::Char('q')])
            .unwrap();
        assert!(matches!(res, SubmitResult::Ok));
        // shutdown joins the writer, guaranteeing the frame was written before we read.
        renderer.shutdown().unwrap();
        drop(write_end);
        let mut read_end = read_end;
        let mut got = Vec::new();
        read_end.read_to_end(&mut got).unwrap();
        assert_eq!(got, b"\x1b[?2026hframe1\x1b[?2026l");
    }

    #[test]
    fn threaded_renderer_shutdown_joins_promptly() {
        let (_read_end, write_end) = make_pipe();
        let fd = write_end.as_raw_fd();
        let quit = Arc::new(AtomicBool::new(false));
        let renderer = ThreadedRenderer::new(quit, fd);
        let start = Instant::now();
        renderer.shutdown().unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "shutdown took {:?}, expected prompt join",
            start.elapsed()
        );
    }

    #[test]
    fn threaded_renderer_reports_write_died_after_drop() {
        let (_read_end, write_end) = make_pipe();
        let fd = write_end.as_raw_fd();
        let quit = Arc::new(AtomicBool::new(false));
        let mut renderer = ThreadedRenderer::new(quit.clone(), fd);
        // Drop the sender to simulate the writer having exited.
        renderer.tx.take();
        let res = renderer
            .submit(vec![b'x'; 10], &quit, &[KeyCode::Char('q')])
            .unwrap();
        assert!(matches!(res, SubmitResult::WriterDied));
    }
}
