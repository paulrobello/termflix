use std::io;
use std::os::unix::io::RawFd;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::os::unix::io::{AsRawFd, FromRawFd};

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
}
