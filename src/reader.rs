use crate::state::{Data, State};
use std::io;
use std::pin::Pin;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// The read half of the pipe
///
/// Implements [`tokio::io::AsyncRead`][tokio-async-read] when feature `tokio` is enabled (the
/// default). Implements [`futures::io::AsyncRead`][futures-async-read] when feature `futures` is
/// enabled.
///
/// [futures-async-read]: https://docs.rs/futures/0.3.5/futures/io/trait.AsyncRead.html
/// [tokio-async-read]: https://docs.rs/tokio/0.2.16/tokio/io/trait.AsyncRead.html
pub struct PipeReader {
    pub(crate) state: Arc<Mutex<State>>,
}

impl PipeReader {
    /// Closes the pipe, any further read will return EOF and any further write will raise an error.
    pub fn close(&self) -> io::Result<()> {
        match self.state.lock() {
            Ok(mut state) => {
                state.closed = true;
                self.wake_writer_half(&*state);
                Ok(())
            }
            Err(err) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "{}: PipeReader: Failed to lock the channel state: {}",
                    env!("CARGO_PKG_NAME"),
                    err
                ),
            )),
        }
    }

    /// It returns true if the next data chunk is written by the writer and consumed by the reader; Otherwise it returns false.
    pub fn is_flushed(&self) -> io::Result<bool> {
        let state = match self.state.lock() {
            Ok(s) => s,
            Err(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "{}: PipeReader: Failed to lock the channel state: {}",
                        env!("CARGO_PKG_NAME"),
                        err
                    ),
                ));
            }
        };

        Ok(state.done_cycle)
    }

    fn wake_writer_half(&self, state: &State) {
        if let Some(ref waker) = state.writer_waker {
            waker.clone().wake();
        }
    }

    fn copy_data_into_buffer(&self, data: &Data, buf: &mut [u8]) -> usize {
        let len = data.len.min(buf.len());
        unsafe {
            ptr::copy_nonoverlapping(data.ptr, buf.as_mut_ptr(), len);
        }
        len
    }

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut state;
        match self.state.lock() {
            Ok(s) => state = s,
            Err(err) => {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "{}: PipeReader: Failed to lock the channel state: {}",
                        env!("CARGO_PKG_NAME"),
                        err
                    ),
                )))
            }
        }

        if state.closed {
            return Poll::Ready(Ok(0));
        }

        return if state.done_cycle {
            state.reader_waker = Some(cx.waker().clone());
            Poll::Pending
        } else {
            if let Some(ref data) = state.data {
                let copied_bytes_len = self.copy_data_into_buffer(data, buf);

                state.data = None;
                state.read = copied_bytes_len;
                state.done_reading = true;
                state.reader_waker = None;

                self.wake_writer_half(&*state);

                Poll::Ready(Ok(copied_bytes_len))
            } else {
                state.reader_waker = Some(cx.waker().clone());
                Poll::Pending
            }
        };
    }
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            log::warn!(
                "{}: PipeReader: Failed to close the channel on drop: {}",
                env!("CARGO_PKG_NAME"),
                err
            );
        }
    }
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncRead for PipeReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_read(cx, buf)
    }
}

#[cfg(feature = "futures")]
impl futures::io::AsyncRead for PipeReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_read(cx, buf)
    }
}
