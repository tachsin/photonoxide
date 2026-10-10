//! A coordinator: hands tasks to workers, takes back the tasks of a worker that died, and
//! gathers the results by index.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::protocol::{self, Message, PROTOCOL, Reader};
use super::{HANDSHAKE, VERSION, farm_error};
use crate::run::{Format, Job, PointFn, Points, Run, Stop};
use crate::{Error, Result};

/// How often a waiting coordinator looks at its stop, its workers and their tasks' times.
const TICK: Duration = Duration::from_millis(50);

/// How a farm hands out its tasks.
#[derive(Clone, Debug)]
pub struct FarmOptions {
    /// The token remote workers expect (local workers get one of their own).
    pub token: Option<String>,
    /// A task's hard limit: past it the task fails with that reason and its worker is dropped
    /// (a local one killed). None by default; a run's own timeout still holds.
    pub task_timeout: Option<Duration>,
    /// How many workers a task may lose (each died or broke off while it had the task) before
    /// it fails with that reason: 3 by default.
    pub attempts: usize,
}

impl Default for FarmOptions {
    fn default() -> FarmOptions {
        FarmOptions {
            token: None,
            task_timeout: None,
            attempts: 3,
        }
    }
}

/// A worker the farm knows: its address and token, and, for a local one, its process.
#[derive(Clone, Debug)]
struct Endpoint {
    address: String,
    token: String,
    local: Option<usize>,
}

/// The local workers' processes, killed when the last holder lets go.
#[derive(Debug, Default)]
struct Locals(Mutex<Vec<Child>>);

impl Locals {
    fn kill(&self, k: usize) {
        if let Some(child) = lock(&self.0).get_mut(k) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Locals {
    fn drop(&mut self) {
        for child in lock(&self.0).iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A farm: workers on this machine or others, each a `photonoxide worker`, and the way tasks
/// are handed to them ([`FarmOptions`]). See [the module](super).
#[derive(Debug)]
pub struct Farm {
    endpoints: Vec<Endpoint>,
    locals: Arc<Locals>,
    options: FarmOptions,
}

impl Farm {
    /// A farm with no workers yet.
    pub fn new(options: FarmOptions) -> Farm {
        Farm {
            endpoints: Vec::new(),
            locals: Arc::new(Locals::default()),
            options,
        }
    }

    /// Adds a worker listening at `address` (`host:port`), which [`FarmOptions::token`] is
    /// given to.
    pub fn add_remote(&mut self, address: &str) {
        self.endpoints.push(Endpoint {
            address: address.to_owned(),
            token: self.options.token.clone().unwrap_or_default(),
            local: None,
        });
    }

    /// Starts `count` workers on this machine: `program worker --listen 127.0.0.1:0 --token
    /// <fresh> --attached`, each with `threads` of rayon's threads (`RAYON_NUM_THREADS`; by
    /// default the machine's divided among them, at least 1), each serving until the farm is
    /// dropped or this process ends. `program` is the `photonoxide` program.
    ///
    /// # Errors
    ///
    /// [`Error::Farm`] if a worker can't be started or doesn't say where it listens within 30
    /// seconds.
    pub fn spawn_local(
        &mut self,
        program: &Path,
        count: usize,
        threads: Option<usize>,
    ) -> Result<()> {
        let token = fresh_token();
        let threads = threads.unwrap_or_else(|| {
            let all = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
            (all / count.max(1)).max(1)
        });
        let mut started = Vec::new();
        for _ in 0..count {
            let mut child = Command::new(program)
                .args(["worker", "--listen", "127.0.0.1:0", "--attached"])
                .env("PHOTONOXIDE_FARM_TOKEN", &token)
                .env("RAYON_NUM_THREADS", threads.to_string())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| farm_error(format!("{}: {e}", program.display())))?;
            let stdout = child.stdout.take();
            let local = {
                let mut all = lock(&self.locals.0);
                all.push(child);
                all.len() - 1
            };
            started.push((local, stdout));
        }
        // each says where it listens, on its first line
        for (local, stdout) in started {
            let Some(stdout) = stdout else {
                return Err(farm_error("a local worker's output can't be read"));
            };
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let mut line = String::new();
                let mut reader = BufReader::new(stdout);
                let _ = reader.read_line(&mut line);
                let _ = tx.send(line);
                // the rest of what it writes, read so that it never blocks on a full pipe
                std::io::copy(&mut reader, &mut std::io::sink()).ok();
            });
            let line = rx
                .recv_timeout(Duration::from_secs(30))
                .map_err(|_| farm_error("a local worker didn't say where it listens"))?;
            let address = line
                .trim()
                .strip_prefix("listening on ")
                .ok_or_else(|| {
                    farm_error(format!(
                        "a local worker said {:?}, not where it listens",
                        line.trim()
                    ))
                })?
                .to_owned();
            self.endpoints.push(Endpoint {
                address,
                token: token.clone(),
                local: Some(local),
            });
        }
        Ok(())
    }

    /// The workers' addresses, local ones first if they were started first.
    pub fn addresses(&self) -> Vec<String> {
        self.endpoints.iter().map(|e| e.address.clone()).collect()
    }

    /// Has `run`'s sweep solve its points on the farm's workers ([`Run::set_points`]). The
    /// workers set `job` up at once, while the run sets it up too; at the sweep, a worker whose
    /// setup recorded other events than the run's is dropped with the reason. The points come
    /// back in their order, as each completes, into the run's record. `stop` is the run's.
    ///
    /// A job with no `[task.sweep]` farms nothing; its run goes as it would.
    pub fn farm_sweep(&self, job: &Job, run: &mut Run, stop: &Stop) {
        if job.task().get("sweep").is_none() {
            return;
        }
        let open = Message::Open {
            job: job.text().to_owned(),
            format: job.format(),
            seconds: stop.remaining().map(|d| d.as_secs_f64()),
        };
        let session = Session::start(
            self,
            Some(open),
            Arc::new(|index| Message::Point { index }),
            stop,
        );
        run.set_points(Box::new(Farmed {
            session: Some(session),
        }));
    }

    /// Runs `jobs` on the farm's workers, each to its record (its events, as the lines its
    /// `events.jsonl` would have), returned in the jobs' order: a job's error is its result. A
    /// job its own timeout or `stop` cut short is still a record, which says why it stopped;
    /// one `stop` came before is an error that says so.
    ///
    /// # Errors
    ///
    /// [`Error::Farm`] if no worker is left with jobs still to run, naming why each was lost.
    pub fn run_jobs(&self, jobs: &[Job], stop: &Stop) -> Result<Vec<Result<Vec<String>>>> {
        let texts: Arc<Vec<(String, Format)>> = Arc::new(
            jobs.iter()
                .map(|j| (j.text().to_owned(), j.format()))
                .collect(),
        );
        let task_stop = stop.clone();
        let session = Session::start(
            self,
            None,
            Arc::new(move |index| Message::Run {
                index,
                job: texts[index].0.clone(),
                format: texts[index].1,
                seconds: task_stop.remaining().map(|d| d.as_secs_f64()),
            }),
            stop,
        );
        session.queue(0..jobs.len(), None);
        let mut results: Vec<Option<Outcome>> = vec![None; jobs.len()];
        let mut left = jobs.len();
        while left > 0 {
            if stop.reason().is_some() {
                break;
            }
            match session.next() {
                Next::Done(index, outcome) => {
                    if results[index].is_none() {
                        left -= 1;
                    }
                    results[index] = Some(outcome);
                }
                Next::Waiting => {}
                Next::NoWorkers(reasons) => {
                    session.finish();
                    return Err(no_workers(&reasons));
                }
            }
        }
        session.finish();
        Ok(results
            .into_iter()
            .enumerate()
            .map(|(k, r)| match r {
                Some(Outcome::Events(lines)) => Ok(lines),
                Some(Outcome::Failed(e)) => Err(farm_error(format!("job {k}: {e}"))),
                Some(Outcome::Stopped) | None => {
                    Err(farm_error(format!("job {k}: stopped before it was done")))
                }
            })
            .collect())
    }
}

fn no_workers(reasons: &[String]) -> Error {
    if reasons.is_empty() {
        farm_error("no workers")
    } else {
        farm_error(format!("no worker left: {}", reasons.join("; ")))
    }
}

/// A token for local workers: 128 bits from the standard library's randomly keyed hasher, the
/// time and the process.
fn fresh_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let half = |salt: u64| {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(now);
        h.write_u32(std::process::id());
        h.write_u64(salt);
        h.finish()
    };
    format!("{:016x}{:016x}", half(1), half(2))
}

/// A task's outcome.
#[derive(Clone, Debug)]
enum Outcome {
    Events(Vec<String>),
    Failed(String),
    Stopped,
}

/// What a session's [`Session::next`] found.
enum Next {
    Done(usize, Outcome),
    Waiting,
    NoWorkers(Vec<String>),
}

/// What a session's workers share.
#[derive(Default)]
struct State {
    queue: VecDeque<usize>,
    /// The workers each task lost, by index.
    lost_by: std::collections::HashMap<usize, usize>,
    /// Tasks queued or in flight, not yet with an outcome.
    pending: usize,
    /// What the workers' setups must have recorded (a sweep's run's digest).
    expected: Option<u64>,
    alive: usize,
    lost: Vec<String>,
    quit: bool,
    /// Whether the tasks were queued: until then a worker with none waits rather than leaves.
    queued: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

type TaskMessage = Arc<dyn Fn(usize) -> Message + Send + Sync>;

/// One use of a farm's workers: a sweep's points or a list of jobs.
struct Session {
    shared: Arc<Shared>,
    outcomes: Receiver<(usize, Outcome)>,
    threads: Vec<JoinHandle<()>>,
    // the local workers, alive while the session is
    _locals: Arc<Locals>,
}

impl Session {
    fn start(farm: &Farm, open: Option<Message>, task: TaskMessage, stop: &Stop) -> Session {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                alive: farm.endpoints.len(),
                ..State::default()
            }),
            wake: Condvar::new(),
        });
        let (tx, outcomes) = mpsc::channel();
        let threads = farm
            .endpoints
            .iter()
            .map(|endpoint| {
                let driver = Driver {
                    endpoint: endpoint.clone(),
                    shared: Arc::clone(&shared),
                    outcomes: tx.clone(),
                    task: Arc::clone(&task),
                    open: open.clone(),
                    stop: stop.clone(),
                    options: farm.options.clone(),
                    locals: Arc::clone(&farm.locals),
                };
                std::thread::spawn(move || driver.drive())
            })
            .collect();
        Session {
            shared,
            outcomes,
            threads,
            _locals: Arc::clone(&farm.locals),
        }
    }

    /// Queues the tasks `indices`; for a sweep, with the digest the workers' setups must have.
    fn queue(&self, indices: std::ops::Range<usize>, expected: Option<u64>) {
        let mut state = lock(&self.shared.state);
        state.pending += indices.len();
        state.queue.extend(indices);
        state.expected = expected;
        state.queued = true;
        drop(state);
        self.shared.wake.notify_all();
    }

    /// The next outcome, if one comes within a tick.
    fn next(&self) -> Next {
        match self.outcomes.recv_timeout(TICK) {
            Ok((index, outcome)) => return Next::Done(index, outcome),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
        }
        let state = lock(&self.shared.state);
        if state.alive == 0 && state.pending > 0 {
            // (an outcome sent just before the last worker went is still to be read)
            drop(state);
            if let Ok((index, outcome)) = self.outcomes.try_recv() {
                return Next::Done(index, outcome);
            }
            return Next::NoWorkers(lock(&self.shared.state).lost.clone());
        }
        Next::Waiting
    }

    /// Tells the workers there is nothing more, and waits for their threads.
    fn finish(mut self) {
        self.stop_threads();
    }

    fn stop_threads(&mut self) {
        lock(&self.shared.state).quit = true;
        self.shared.wake.notify_all();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop_threads();
    }
}

/// A run's sweep, farmed.
struct Farmed {
    session: Option<Session>,
}

impl std::fmt::Debug for Farmed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Farmed").finish_non_exhaustive()
    }
}

impl Points for Farmed {
    fn sweep(
        &mut self,
        run: &mut Run,
        stop: &Stop,
        points: std::ops::Range<usize>,
        _point: &PointFn<'_>,
    ) -> Result<()> {
        // (the session's workers are let go when it drops, on every way out)
        let Some(session) = self.session.take() else {
            return Err(farm_error("a farmed run sweeps once"));
        };
        let first = points.start;
        let n = points.len();
        session.queue(points, Some(run.digest()));
        let mut results: Vec<Option<Outcome>> = vec![None; n];
        let mut next = 0;
        loop {
            // the points done, in order, as far as they go: the record a loop over them writes
            while next < n {
                match results[next].take() {
                    None => break,
                    Some(Outcome::Events(lines)) => {
                        for line in &lines {
                            run.record_line(line)?;
                        }
                    }
                    Some(Outcome::Failed(e)) => {
                        return Err(farm_error(format!("point {}: {e}", first + next)));
                    }
                    Some(Outcome::Stopped) => return Ok(()),
                }
                next += 1;
            }
            if next == n || stop.reason().is_some() {
                return Ok(());
            }
            match session.next() {
                Next::Done(index, outcome) => results[index - first] = Some(outcome),
                Next::Waiting => {}
                Next::NoWorkers(reasons) => return Err(no_workers(&reasons)),
            }
        }
    }
}

/// A worker's thread in a session: connects, sets a sweep up if there is one, then keeps the
/// worker busy with tasks until there are none, the farm quits or the worker is lost.
struct Driver {
    endpoint: Endpoint,
    shared: Arc<Shared>,
    outcomes: Sender<(usize, Outcome)>,
    task: TaskMessage,
    open: Option<Message>,
    stop: Stop,
    options: FarmOptions,
    locals: Arc<Locals>,
}

/// Why a worker is lost.
struct Lost(String);

impl Driver {
    fn drive(self) {
        let mut in_flight: Vec<(usize, Instant)> = Vec::new();
        let lost = match self.connect() {
            Ok((mut stream, reader, slots)) => {
                let done = self.work(&mut stream, reader, slots, &mut in_flight);
                if done.is_ok() {
                    let _ = protocol::send(&mut stream, &Message::Close);
                }
                let _ = stream.shutdown(Shutdown::Both);
                done.err()
            }
            Err(lost) => Some(lost),
        };
        let mut state = lock(&self.shared.state);
        state.alive -= 1;
        if let Some(Lost(reason)) = lost {
            if let Some(k) = self.endpoint.local {
                self.locals.kill(k);
            }
            // its tasks go to the others, unless they have lost too many workers
            for (index, _) in in_flight {
                let count = state.lost_by.entry(index).or_insert(0);
                *count += 1;
                let count = *count;
                if count >= self.options.attempts.max(1) {
                    state.pending -= 1;
                    let _ = self.outcomes.send((
                        index,
                        Outcome::Failed(format!(
                            "lost {count} workers while they had it, the last {}: {reason}",
                            self.endpoint.address
                        )),
                    ));
                } else {
                    state.queue.push_front(index);
                }
            }
            state
                .lost
                .push(format!("{}: {reason}", self.endpoint.address));
        }
        drop(state);
        self.shared.wake.notify_all();
    }

    /// A task's outcome, final: it is no longer pending.
    fn settle(&self, index: usize, outcome: Outcome) {
        let mut state = lock(&self.shared.state);
        state.pending -= 1;
        let _ = self.outcomes.send((index, outcome));
        drop(state);
        self.shared.wake.notify_all();
    }

    /// Why a task past `limit` failed.
    fn late(&self, limit: Duration) -> String {
        format!(
            "past its timeout of {} s on {}",
            limit.as_secs_f64(),
            self.endpoint.address
        )
    }

    fn quitting(&self) -> bool {
        lock(&self.shared.state).quit || self.stop.reason().is_some()
    }

    fn connect(&self) -> std::result::Result<(TcpStream, Reader, usize), Lost> {
        let lost = |e: String| Lost(e);
        let address = self
            .endpoint
            .address
            .to_socket_addrs()
            .map_err(|e| lost(e.to_string()))?
            .next()
            .ok_or_else(|| lost("no such address".into()))?;
        let mut stream =
            TcpStream::connect_timeout(&address, HANDSHAKE).map_err(|e| lost(e.to_string()))?;
        let _ = stream.set_nodelay(true);
        let mut reader = Reader::new(stream.try_clone().map_err(|e| lost(e.to_string()))?);
        stream
            .set_read_timeout(Some(HANDSHAKE))
            .map_err(|e| lost(e.to_string()))?;
        protocol::send(
            &mut stream,
            &Message::Hello {
                protocol: PROTOCOL,
                photonoxide: VERSION.into(),
                token: self.endpoint.token.clone(),
            },
        )
        .map_err(lost)?;
        let slots = match reader.wait().map_err(lost)? {
            Message::Welcome {
                protocol,
                photonoxide,
                slots,
            } if protocol == PROTOCOL && photonoxide == VERSION => slots.max(1),
            Message::Welcome {
                protocol,
                photonoxide,
                ..
            } => {
                return Err(lost(format!(
                    "a worker of protocol {protocol} and photonoxide {photonoxide}, not \
                     {PROTOCOL} and {VERSION}"
                )));
            }
            Message::Refused { reason } => return Err(lost(format!("refused: {reason}"))),
            other => return Err(lost(format!("expected a welcome, got {other:?}"))),
        };
        // from here a read waits a tick at most
        stream
            .set_read_timeout(Some(TICK))
            .map_err(|e| lost(e.to_string()))?;
        Ok((stream, reader, slots))
    }

    /// A sweep's setup on the worker: `Ok(false)` when the farm quit first.
    fn open(
        &self,
        stream: &mut TcpStream,
        reader: &mut Reader,
        open: &Message,
    ) -> std::result::Result<bool, Lost> {
        protocol::send(stream, open).map_err(Lost)?;
        let since = Instant::now();
        let digest = loop {
            match reader.next().map_err(Lost)? {
                Some(Message::Ready { digest }) => break digest,
                Some(Message::Failed { error, .. }) => {
                    return Err(Lost(format!("couldn't set the job up: {error}")));
                }
                Some(other) => return Err(Lost(format!("expected ready, got {other:?}"))),
                None => {}
            }
            if self.quitting() {
                return Ok(false);
            }
            if let Some(limit) = self.options.task_timeout
                && since.elapsed() > limit
            {
                return Err(Lost(format!(
                    "not set up within the task timeout of {} s",
                    limit.as_secs_f64()
                )));
            }
        };
        // compared with the run's own setup when its sweep starts
        let mut state = lock(&self.shared.state);
        loop {
            if state.quit || self.stop.reason().is_some() {
                return Ok(false);
            }
            if let Some(expected) = state.expected {
                if expected != digest {
                    return Err(Lost(
                        "it set the job up otherwise than the run did: its record before the \
                         sweep differs (another direct solver chosen by auto, or another build?)"
                            .into(),
                    ));
                }
                return Ok(true);
            }
            state = self
                .shared
                .wake
                .wait_timeout(state, TICK)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
    }

    fn work(
        &self,
        stream: &mut TcpStream,
        mut reader: Reader,
        slots: usize,
        in_flight: &mut Vec<(usize, Instant)>,
    ) -> std::result::Result<(), Lost> {
        if let Some(open) = &self.open
            && !self.open(stream, &mut reader, open)?
        {
            return Ok(());
        }
        loop {
            // as many tasks as it takes at once
            let mut taken = Vec::new();
            {
                let mut state = lock(&self.shared.state);
                if state.quit || self.stop.reason().is_some() {
                    return Ok(());
                }
                while in_flight.len() + taken.len() < slots {
                    let Some(index) = state.queue.pop_front() else {
                        break;
                    };
                    taken.push(index);
                }
                if in_flight.is_empty() && taken.is_empty() {
                    if state.queued && state.pending == 0 {
                        return Ok(());
                    }
                    let _ = self
                        .shared
                        .wake
                        .wait_timeout(state, TICK)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    continue;
                }
            }
            for index in taken {
                // in flight before it is sent: if the sending fails, it goes back to the queue
                in_flight.push((index, Instant::now()));
                protocol::send(stream, &(self.task)(index)).map_err(Lost)?;
            }
            // an answer, a tick at a time
            let answered = match reader.next().map_err(Lost)? {
                Some(Message::Events { index, events }) => Some((index, Outcome::Events(events))),
                Some(Message::Failed {
                    index: Some(index),
                    error,
                }) => Some((index, Outcome::Failed(error))),
                Some(Message::Stopped { index }) => Some((index, Outcome::Stopped)),
                Some(other) => return Err(Lost(format!("unexpected {other:?}"))),
                None => None,
            };
            if let Some((index, outcome)) = answered {
                let Some(at) = in_flight.iter().position(|&(i, _)| i == index) else {
                    return Err(Lost(format!("an answer to task {index}, not its own")));
                };
                let (_, sent) = in_flight.remove(at);
                // an answer that came after the limit is late all the same (its worker isn't
                // stuck, and stays)
                let outcome = match self.options.task_timeout {
                    Some(limit) if sent.elapsed() > limit => Outcome::Failed(self.late(limit)),
                    _ => outcome,
                };
                self.settle(index, outcome);
                continue;
            }
            // a task past its time fails, and its worker is dropped (it may be stuck)
            if let Some(limit) = self.options.task_timeout
                && let Some(at) = in_flight.iter().position(|(_, t)| t.elapsed() > limit)
            {
                let (index, _) = in_flight.remove(at);
                self.settle(index, Outcome::Failed(self.late(limit)));
                return Err(Lost(format!(
                    "task {index} past its timeout of {} s",
                    limit.as_secs_f64()
                )));
            }
        }
    }
}
