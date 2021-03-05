//! `io::Write` streamer for slog-rs
//!
//! One of the main drains not included in the core `slog-rs` create.
//! `Streamer` drain serializes logging records into stream of bytes
//! using given `Format` and writes it to a given `io::Write`.
#![warn(missing_docs)]

use slog::{Drain, DrainExt};

use std::cell::RefCell;

use slog::Record;
use std::io;
use std::sync::Mutex;

use slog::OwnedKeyValueList;
use slog_extra::Async;

include!("format.rs");

thread_local! {
    static TL_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(128))
}

/// Drain formating records and writing them to a byte-stream (`io::Write`)
///
/// Uses mutex to serialize writes to `io`. Use `AsyncStreamer` for better
/// performance, but without guarantee of immediate output.
pub struct Streamer<W: io::Write, F: Format> {
    io: Mutex<W>,
    format: F,
}

impl<W: io::Write, F: Format> Streamer<W, F> {
    /// Create new `Streamer` writing to `io` using `format`
    pub fn new(io: W, format: F) -> Self {
        Streamer {
            io: Mutex::new(io),
            format: format,
        }
    }
}

impl<W: 'static + io::Write + Send, F: Format + Send> Drain for Streamer<W, F> {
    type Error = io::Error;

    fn log(&self, info: &Record, logger_values: &OwnedKeyValueList) -> io::Result<()> {
        TL_BUF.with(|buf| {
            let mut buf = buf.borrow_mut();
            let res = {
                || {
                    self.format.format(&mut *buf, info, logger_values)?;
                    {
                        let mut io = self
                            .io
                            .lock()
                            .map_err(|_| io::Error::new(io::ErrorKind::Other, "lock error"))?;
                        io.write_all(&buf)?;
                    }
                    Ok(())
                }
            }();
            buf.clear();
            res
        })
    }
}

/// Stream logging records to IO
///
/// Create `Streamer` drain
pub fn stream<W: io::Write + Send, F: Format>(io: W, format: F) -> Streamer<W, F> {
    Streamer::new(io, format)
}

/// Stream logging records to IO asynchronously
///
/// Create `AsyncStreamer` drain
pub fn async_stream<W: io::Write + Send + 'static, F: Format + Send + 'static>(
    io: W,
    format: F,
) -> Async {
    Async::new(Streamer::new(io, format).fuse())
}
