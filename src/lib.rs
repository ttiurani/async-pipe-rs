//! Creates an asynchronous piped reader and writer pair using `tokio.rs` and `futures`.
//!
//! # Examples
//!
//! ```
//! # async fn run() {
//! use async_pipe;
//! use tokio::prelude::*;
//!
//! let (mut w, mut r) = async_pipe::pipe();
//!  
//! tokio::spawn(async move {
//!     w.write_all(b"hello world").await.unwrap();
//! });
//!  
//! let mut v = Vec::new();
//! r.read_to_end(&mut v).await.unwrap();
//!
//! println!("Received: {:?}", String::from_utf8(v));
//! # }
//!
//! tokio::runtime::Runtime::new().unwrap().block_on(run());
//! ```
//!
//! # Featues
//!
//! * `tokio` (default) Implement `AsyncWrite` and `AsyncRead` from `tokio::io`.
//! * `futures` Implement `AsyncWrite` and `AsyncRead` from `futures::io`

use state::State;
use std::sync::{Arc, Mutex};

pub use self::reader::PipeReader;
pub use self::writer::PipeWriter;

mod reader;
mod state;
mod writer;

/// Creates a piped pair of an [`AsyncWrite`](https://docs.rs/tokio/0.2.16/tokio/io/trait.AsyncWrite.html) and an [`AsyncRead`](https://docs.rs/tokio/0.2.15/tokio/io/trait.AsyncRead.html).
pub fn pipe() -> (PipeWriter, PipeReader) {
    let shared_state = Arc::new(Mutex::new(State {
        reader_waker: None,
        writer_waker: None,
        data: None,
        done_reading: false,
        read: 0,
        done_cycle: true,
        closed: false,
    }));

    let w = PipeWriter {
        state: Arc::clone(&shared_state),
    };

    let r = PipeReader {
        state: Arc::clone(&shared_state),
    };

    (w, r)
}
