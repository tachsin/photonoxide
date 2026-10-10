//! A worker: serves coordinators, one at a time, running the jobs and solving the sweep points
//! they send.

use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::protocol::{self, Message, PROTOCOL, Reader};
use super::{HANDSHAKE, VERSION, farm_error, same_token};
use crate::Result;
use crate::run::{Job, PointFn, Points, Run, Stop};

/// How a worker serves.
#[derive(Clone, Debug, Default)]
pub struct ServeOptions {
    /// The token a coordinator must give. Required to listen on an address beyond this machine.
    pub token: Option<String>,
    /// How many tasks (points or jobs) it takes at once, each on its own thread: 1 when 0.
    pub slots: usize,
    /// Called with each job before it is set up or run: the program gives `auto` this
    /// machine's benchmark records there, and registers the external libraries a job needs.
    pub prepare: Option<fn(&Job)>,
}

/// A worker's listening socket.
#[derive(Debug)]
pub struct Server {
    listener: TcpListener,
    options: ServeOptions,
}

/// What a worker did for one coordinator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Served {
    /// The coordinator's address.
    pub peer: String,
    /// The tasks (points or jobs) it answered.
    pub tasks: usize,
    /// Why the coordinator was refused or the session ended early, if it was.
    pub problem: Option<String>,
}

impl Server {
    /// A worker listening on `address` (e.g. `"127.0.0.1:7878"`, or port 0 for any free one).
    ///
    /// # Errors
    ///
    /// [`crate::Error::Farm`] if the address can't be bound, or is beyond this machine (not a
    /// loopback address) with no token.
    pub fn bind(address: &str, options: ServeOptions) -> Result<Server> {
        let addresses: Vec<SocketAddr> = address
            .to_socket_addrs()
            .map_err(|e| farm_error(format!("{address}: {e}")))?
            .collect();
        if options.token.as_deref().is_none_or(str::is_empty)
            && addresses.iter().any(|a| !a.ip().is_loopback())
        {
            return Err(farm_error(format!(
                "listening on {address}, beyond this machine, needs a token (--token or \
                 PHOTONOXIDE_FARM_TOKEN)"
            )));
        }
        let listener =
            TcpListener::bind(&addresses[..]).map_err(|e| farm_error(format!("{address}: {e}")))?;
        Ok(Server { listener, options })
    }

    /// The address it listens on.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Farm`] if the system can't say.
    pub fn address(&self) -> Result<SocketAddr> {
        self.listener
            .local_addr()
            .map_err(|e| farm_error(e.to_string()))
    }

    /// Waits for a coordinator and serves it until it closes the connection.
    ///
    /// # Errors
    ///
    /// [`crate::Error::Farm`] if no connection can be accepted.
    pub fn serve_one(&self) -> Result<Served> {
        let (stream, peer) = self
            .listener
            .accept()
            .map_err(|e| farm_error(e.to_string()))?;
        Ok(serve(stream, peer.to_string(), &self.options))
    }
}

// one coordinator's connection
fn serve(stream: TcpStream, peer: String, options: &ServeOptions) -> Served {
    let mut served = Served {
        peer,
        tasks: 0,
        problem: None,
    };
    let _ = stream.set_nodelay(true);
    let (mut out, mut reader) = match stream.try_clone() {
        Ok(s) => (s, Reader::new(stream)),
        Err(e) => {
            served.problem = Some(e.to_string());
            return served;
        }
    };
    // the hello, within the handshake's time
    let _ = out.set_read_timeout(Some(HANDSHAKE));
    reader.limit(protocol::LONGEST_HELLO);
    let hello = reader.wait();
    reader.limit(protocol::LONGEST);
    let _ = out.set_read_timeout(None);
    let refused = match hello {
        Ok(Message::Hello {
            protocol,
            photonoxide,
            token,
        }) => {
            if protocol != PROTOCOL {
                Some(format!(
                    "the coordinator speaks protocol {protocol}, this worker {PROTOCOL}"
                ))
            } else if photonoxide != VERSION {
                Some(format!(
                    "the coordinator is photonoxide {photonoxide}, this worker {VERSION}: a \
                     farm's workers must be the same version, for the same bits"
                ))
            } else if let Some(mine) = options.token.as_deref().filter(|t| !t.is_empty())
                && !same_token(mine, &token)
            {
                Some("wrong token".into())
            } else {
                None
            }
        }
        Ok(other) => Some(format!("expected a hello, got {other:?}")),
        Err(e) => Some(e),
    };
    if let Some(reason) = refused {
        let _ = protocol::send(
            &mut out,
            &Message::Refused {
                reason: reason.clone(),
            },
        );
        served.problem = Some(reason);
        return served;
    }
    let slots = options.slots.max(1);
    if let Err(e) = protocol::send(
        &mut out,
        &Message::Welcome {
            protocol: PROTOCOL,
            photonoxide: VERSION.into(),
            slots,
        },
    ) {
        served.problem = Some(e);
        return served;
    }
    // the connection's messages, read on their own thread; its end stops what runs
    let gone = Stop::new(None);
    let (tx, rx) = mpsc::channel();
    {
        let gone = gone.clone();
        std::thread::spawn(move || {
            while let Ok(m) = reader.wait() {
                if tx.send(m).is_err() {
                    break;
                }
            }
            gone.request();
        });
    }
    let inbox = Arc::new(Inbox {
        rx: Mutex::new(rx),
        first: Mutex::new(None),
        closing: AtomicBool::new(false),
    });
    let out = Arc::new(Mutex::new(out));
    let answered = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    match inbox.next() {
        Some(Message::Open {
            job,
            format,
            seconds,
        }) => {
            served.problem = sweep(
                &job, format, seconds, options, &gone, &inbox, &out, &answered,
            );
        }
        Some(m @ Message::Run { .. }) => {
            *lock(&inbox.first) = Some(m);
            jobs(slots, options, &gone, &inbox, &out, &answered);
        }
        Some(Message::Close) | None => {}
        Some(other) => served.problem = Some(format!("unexpected {other:?}")),
    }
    served.tasks = answered.load(Ordering::Relaxed);
    // the coordinator closes; a worker that still holds the connection lets it go
    if let Ok(s) = out.lock() {
        let _ = s.shutdown(std::net::Shutdown::Both);
    }
    served
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The messages the reader thread passed on, for the threads that take tasks.
struct Inbox {
    rx: Mutex<Receiver<Message>>,
    first: Mutex<Option<Message>>,
    closing: AtomicBool,
}

impl Inbox {
    /// The next message; `None` once the coordinator closed or went away.
    fn next(&self) -> Option<Message> {
        if let Some(m) = lock(&self.first).take() {
            return Some(m);
        }
        loop {
            if self.closing.load(Ordering::Relaxed) {
                return None;
            }
            match lock(&self.rx).recv_timeout(Duration::from_millis(100)) {
                Ok(Message::Close) => {
                    self.closing.store(true, Ordering::Relaxed);
                    return None;
                }
                Ok(m) => return Some(m),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.closing.store(true, Ordering::Relaxed);
                    return None;
                }
            }
        }
    }
}

type Out = Arc<Mutex<TcpStream>>;

fn reply(out: &Out, message: &Message) {
    // a reply that can't be sent: the coordinator is gone, which the reader sees
    let _ = protocol::send(&mut lock(out), message);
}

/// The stop of a task: the connection's, within the time the coordinator left and the job's
/// own limit.
fn stop_for(gone: &Stop, seconds: Option<f64>, job: &Job) -> Stop {
    let left = seconds
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(Duration::from_secs_f64);
    gone.within(left).within(job.timeout())
}

/// A sweep's session: the job set up as the coordinator's run sets it up, then its points.
#[allow(clippy::too_many_arguments)]
fn sweep(
    text: &str,
    format: crate::run::Format,
    seconds: Option<f64>,
    options: &ServeOptions,
    gone: &Stop,
    inbox: &Arc<Inbox>,
    out: &Out,
    answered: &Arc<std::sync::atomic::AtomicUsize>,
) -> Option<String> {
    let job = match Job::parse_as(text, format) {
        Ok(j) => j,
        Err(e) => {
            let error = e.to_string();
            reply(
                out,
                &Message::Failed {
                    index: None,
                    error: error.clone(),
                },
            );
            return Some(error);
        }
    };
    if let Some(prepare) = options.prepare {
        prepare(&job);
    }
    let stop = stop_for(gone, seconds, &job);
    let reached = Arc::new(AtomicBool::new(false));
    let mut run = Run::discarding();
    run.set_points(Box::new(Serving {
        slots: options.slots.max(1),
        inbox: Arc::clone(inbox),
        out: Arc::clone(out),
        reached: Arc::clone(&reached),
        answered: Arc::clone(answered),
    }));
    let done = crate::job::execute(&job, &mut run, &stop);
    if reached.load(Ordering::Relaxed) {
        return None;
    }
    // set up, it never came to its sweep
    let error = match done {
        Err(e) => e.to_string(),
        Ok(()) if stop.reason().is_some() => "stopped before its sweep".into(),
        Ok(()) => "the job has no sweep to farm".into(),
    };
    reply(
        out,
        &Message::Failed {
            index: None,
            error: error.clone(),
        },
    );
    Some(error)
}

/// A worker's side of a run's sweep: the points the coordinator asks for, solved here.
struct Serving {
    slots: usize,
    inbox: Arc<Inbox>,
    out: Out,
    reached: Arc<AtomicBool>,
    answered: Arc<std::sync::atomic::AtomicUsize>,
}

impl std::fmt::Debug for Serving {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Serving")
            .field("slots", &self.slots)
            .finish_non_exhaustive()
    }
}

impl Points for Serving {
    fn sweep(
        &mut self,
        run: &mut Run,
        _stop: &Stop,
        points: std::ops::Range<usize>,
        point: &PointFn<'_>,
    ) -> Result<()> {
        self.reached.store(true, Ordering::Relaxed);
        reply(
            &self.out,
            &Message::Ready {
                digest: run.digest(),
            },
        );
        std::thread::scope(|scope| {
            for _ in 0..self.slots {
                scope.spawn(|| {
                    while let Some(m) = self.inbox.next() {
                        let answer = match m {
                            Message::Point { index } if points.contains(&index) => {
                                match point(index) {
                                    Ok(Some(events)) => lines(&events).map_or_else(
                                        |error| Message::Failed {
                                            index: Some(index),
                                            error,
                                        },
                                        |events| Message::Events { index, events },
                                    ),
                                    Ok(None) => Message::Stopped { index },
                                    Err(e) => Message::Failed {
                                        index: Some(index),
                                        error: e.to_string(),
                                    },
                                }
                            }
                            Message::Point { index } => Message::Failed {
                                index: Some(index),
                                error: format!(
                                    "no point {index}: the sweep's are {} to {}",
                                    points.start,
                                    points.end.saturating_sub(1)
                                ),
                            },
                            other => Message::Failed {
                                index: None,
                                error: format!("expected a point, got {other:?}"),
                            },
                        };
                        self.answered.fetch_add(1, Ordering::Relaxed);
                        reply(&self.out, &answer);
                    }
                });
            }
        });
        Ok(())
    }
}

/// Events as the lines [`Run::record`] writes.
fn lines(events: &[crate::job::Event]) -> std::result::Result<Vec<String>, String> {
    events
        .iter()
        .map(|e| serde_json::to_string(e).map_err(|e| e.to_string()))
        .collect()
}

/// A session of whole jobs, `slots` at a time.
fn jobs(
    slots: usize,
    options: &ServeOptions,
    gone: &Stop,
    inbox: &Arc<Inbox>,
    out: &Out,
    answered: &Arc<std::sync::atomic::AtomicUsize>,
) {
    std::thread::scope(|scope| {
        for _ in 0..slots {
            scope.spawn(|| {
                while let Some(m) = inbox.next() {
                    let answer = match m {
                        Message::Run {
                            index,
                            job,
                            format,
                            seconds,
                        } => run_job(index, &job, format, seconds, options, gone),
                        other => Message::Failed {
                            index: None,
                            error: format!("expected a job, got {other:?}"),
                        },
                    };
                    answered.fetch_add(1, Ordering::Relaxed);
                    reply(out, &answer);
                }
            });
        }
    });
}

fn run_job(
    index: usize,
    text: &str,
    format: crate::run::Format,
    seconds: Option<f64>,
    options: &ServeOptions,
    gone: &Stop,
) -> Message {
    let failed = |error: String| Message::Failed {
        index: Some(index),
        error,
    };
    let job = match Job::parse_as(text, format) {
        Ok(j) => j,
        Err(e) => return failed(e.to_string()),
    };
    if let Some(prepare) = options.prepare {
        prepare(&job);
    }
    let stop = stop_for(gone, seconds, &job);
    let mut run = Run::in_memory();
    match crate::job::execute(&job, &mut run, &stop) {
        // a job its stop cut short is still its record, which says why it stopped
        Ok(()) => Message::Events {
            index,
            events: run.lines().to_vec(),
        },
        Err(e) => failed(e.to_string()),
    }
}
