//! The farm's protocol: messages as JSON (serde), each after its length.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;

use serde::{Deserialize, Serialize};

use crate::run::Format;

/// The protocol's version: a coordinator and a worker of another are refused.
pub const PROTOCOL: u32 = 1;

/// The longest message, in bytes: 256 MiB.
pub(crate) const LONGEST: usize = 256 << 20;

/// A message between a coordinator and a worker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Message {
    /// Coordinator: the first message, with the coordinator's protocol, photonoxide version and
    /// token ("" for none).
    Hello {
        protocol: u32,
        photonoxide: String,
        token: String,
    },
    /// Worker: accepted, and how many tasks it takes at once.
    Welcome {
        protocol: u32,
        photonoxide: String,
        slots: usize,
    },
    /// Worker: refused, and why; it closes the connection.
    Refused { reason: String },
    /// Coordinator: a sweep's job, to set up once; `seconds` is the time left, if limited.
    Open {
        job: String,
        format: Format,
        seconds: Option<f64>,
    },
    /// Worker: the sweep's job is set up, and the digest of the events it recorded doing it.
    Ready { digest: u64 },
    /// Coordinator: solve the sweep's point `index`.
    Point { index: usize },
    /// Coordinator: run a whole job, task `index`; `seconds` is the time left, if limited.
    Run {
        index: usize,
        job: String,
        format: Format,
        seconds: Option<f64>,
    },
    /// Worker: the events of task or point `index`, each the line `events.jsonl` would have.
    Events { index: usize, events: Vec<String> },
    /// Worker: task or point `index` failed, or (no index) the sweep's job couldn't be set up.
    Failed { index: Option<usize>, error: String },
    /// Worker: its stop came before task or point `index` was done.
    Stopped { index: usize },
    /// Coordinator: no more tasks; the connection closes after it.
    Close,
}

/// Writes `message` with its length (4 bytes, big-endian) in one write.
pub(crate) fn send(stream: &mut TcpStream, message: &Message) -> Result<(), String> {
    let body = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    if body.len() > LONGEST {
        return Err(format!(
            "a message of {} bytes, more than the protocol's {LONGEST}",
            body.len()
        ));
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    stream
        .write_all(&frame)
        .and_then(|()| stream.flush())
        .map_err(|e| e.to_string())
}

/// Reads messages from a stream, keeping a message that came in part until the rest comes.
pub(crate) struct Reader {
    stream: TcpStream,
    buffer: Vec<u8>,
    longest: usize,
}

/// The longest hello a worker reads from a connection not yet known to be a coordinator's.
pub(crate) const LONGEST_HELLO: usize = 64 << 10;

impl Reader {
    pub(crate) fn new(stream: TcpStream) -> Reader {
        Reader {
            stream,
            buffer: Vec::new(),
            longest: LONGEST,
        }
    }

    /// Refuses messages longer than `longest` bytes from now on (at most [`LONGEST`]).
    pub(crate) fn limit(&mut self, longest: usize) {
        self.longest = longest.min(LONGEST);
    }

    /// The next message, or `None` when none came within the stream's read timeout. An error
    /// when the stream closed, broke, or sent what isn't a message.
    pub(crate) fn next(&mut self) -> Result<Option<Message>, String> {
        let mut chunk = [0u8; 1 << 16];
        loop {
            if self.buffer.len() >= 4 {
                let length = u32::from_be_bytes([
                    self.buffer[0],
                    self.buffer[1],
                    self.buffer[2],
                    self.buffer[3],
                ]) as usize;
                if length > self.longest {
                    return Err(format!(
                        "a message of {length} bytes, more than the {} allowed",
                        self.longest
                    ));
                }
                if self.buffer.len() >= 4 + length {
                    let message = serde_json::from_slice(&self.buffer[4..4 + length])
                        .map_err(|e| format!("not a message of the protocol: {e}"));
                    self.buffer.drain(..4 + length);
                    return message.map(Some);
                }
            }
            match self.stream.read(&mut chunk) {
                Ok(0) => return Err("the connection closed".into()),
                Ok(n) => self.buffer.extend_from_slice(&chunk[..n]),
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    return Ok(None);
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    /// The next message, waiting for it however long it takes (the stream's read timeout
    /// aside, which ends the wait with an error).
    pub(crate) fn wait(&mut self) -> Result<Message, String> {
        self.next()?
            .ok_or_else(|| "no message within the time allowed".to_owned())
    }
}
