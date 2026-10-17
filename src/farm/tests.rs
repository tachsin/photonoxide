use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::*;
use crate::job::execute;
use crate::run::{Job, Run, Stop};

/// A fresh directory under the system's temporary directory, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> TempDir {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("photonoxide-farm-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const TOKEN: &str = "a token for the tests";

/// `count` workers in this process, each on its own port of 127.0.0.1 and taking `slots`
/// tasks at once, serving one coordinator after another for as long as the tests run.
fn workers(count: usize, slots: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            let server = Server::bind(
                "127.0.0.1:0",
                ServeOptions {
                    token: Some(TOKEN.into()),
                    slots,
                    prepare: None,
                },
            )
            .unwrap();
            let address = server.address().unwrap().to_string();
            std::thread::spawn(move || while server.serve_one().is_ok() {});
            address
        })
        .collect()
}

fn farm_of(addresses: &[String], options: FarmOptions) -> Farm {
    let mut farm = Farm::new(FarmOptions {
        token: Some(TOKEN.into()),
        ..options
    });
    for a in addresses {
        farm.add_remote(a);
    }
    farm
}

/// The lines of a run's `events.jsonl`.
fn lines_of(run: &Run) -> Vec<String> {
    std::fs::read_to_string(run.dir().join("events.jsonl"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The job run in this process, its result and its record.
fn alone(text: &str, stop: &Stop) -> (crate::Result<()>, Vec<String>) {
    let root = TempDir::new("alone");
    let job = Job::parse(text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let done = execute(&job, &mut run, stop);
    (done, lines_of(&run))
}

/// The job run with its sweep farmed, its result and its record.
fn farmed(text: &str, farm: &Farm, stop: &Stop) -> (crate::Result<()>, Vec<String>) {
    let root = TempDir::new("farmed");
    let job = Job::parse(text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    farm.farm_sweep(&job, &mut run, stop);
    let done = execute(&job, &mut run, stop);
    (done, lines_of(&run))
}

/// A record without the time its `finished` event gives: the one thing two runs of a job may
/// record differently.
fn timeless(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            if l.starts_with("{\"type\":\"finished\"") {
                let mut v: serde_json::Value = serde_json::from_str(l).unwrap();
                v["seconds"] = serde_json::Value::Null;
                v.to_string()
            } else {
                l.clone()
            }
        })
        .collect()
}

const MODES: &str = r#"
name = "farmed-modes"

[task]
kind = "modes"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
propagation = "x"
y_um = [-1.0, 1.0]
step_nm = 25.0
modes = 2

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [10.0, 0.5]

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = 6
"#;

const FDFD: &str = r#"
name = "farmed-fdfd"

[task]
kind = "fdfd"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-1.4, 1.4]
y_um = [-1.4, 1.4]
step_nm = 40.0

[solver]
direct = "photonoxide"

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [4.0, 0.5]

[[task.port]]
x_um = -0.4
side = "left"

[[task.port]]
x_um = 0.4
side = "right"

[task.sweep]
parameter = "wavelength"
from = 1.5
to = 1.6
points = 6
"#;

/// A job's text without its sweep (the last table of the jobs here).
fn unswept(text: &str) -> &str {
    text.split("[task.sweep]").next().unwrap_or(text)
}

/// The record's lines of sweep points: `sweep_point` events of a modes job, `s_parameters` of
/// an FDFD job's points after the first.
fn points_in(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|l| {
            l.starts_with("{\"type\":\"sweep_point\"")
                || l.starts_with("{\"type\":\"s_parameters\"")
        })
        .count()
}

#[test]
fn a_farmed_sweep_is_the_record_of_one_process_on_any_number_of_workers() {
    // the [solver] table is the FDFD job's own: a job naming no direct solver lets auto choose
    // on each machine
    let fdfd = FDFD.replace("[solver]\ndirect = \"photonoxide\"\n", "");
    for (text, points) in [(MODES, 6), (FDFD, 6), (fdfd.as_str(), 6)] {
        let stop = Stop::new(None);
        let (done, one) = alone(text, &stop);
        done.unwrap();
        assert_eq!(points_in(&one), points);
        let one = timeless(&one);
        for (count, slots) in [(1, 1), (2, 1), (4, 1), (8, 1), (3, 2)] {
            let farm = farm_of(&workers(count, slots), FarmOptions::default());
            let (done, lines) = farmed(text, &farm, &stop);
            done.unwrap();
            assert_eq!(
                timeless(&lines),
                one,
                "{count} workers of {slots} slots: {text}"
            );
        }
    }
}

/// A device's study (crate::job::study_from_device): a coupler's gap and length at its analytic
/// fidelity, and a waveguide's corners.
const STUDIES: [&str; 2] = [
    r#"
name = "farmed-study"

[device]
kind = "directional coupler"

[task]
kind = "sweep"
from_um = 1.53
to_um = 1.57
points = 5

[[task.axis]]
parameter = "gap"
from = 0.15
to = 0.3
points = 4

[[task.axis]]
parameter = "length"
from = 5.0
to = 20.0
points = 3
"#,
    r#"
name = "farmed-corners"

[device]
kind = "waveguide"

[task]
kind = "corners"

[[task.corner]]
name = "wide"
offsets = { width = 0.02 }

[[task.corner]]
name = "thin"
offsets = { thickness = -0.01 }
"#,
];

#[test]
fn a_farmed_study_is_the_record_of_one_process_on_any_number_of_workers() {
    for (text, points) in STUDIES.iter().zip([12, 3]) {
        let stop = Stop::new(None);
        let (done, one) = alone(text, &stop);
        done.unwrap();
        let in_record = one
            .iter()
            .filter(|l| l.starts_with("{\"type\":\"device_point\""))
            .count();
        assert_eq!(in_record, points);
        let one = timeless(&one);
        for (count, slots) in [(1, 1), (3, 1), (2, 2)] {
            let farm = farm_of(&workers(count, slots), FarmOptions::default());
            let (done, lines) = farmed(text, &farm, &stop);
            done.unwrap();
            assert_eq!(timeless(&lines), one, "{count} workers of {slots} slots");
        }
    }
}

/// A connection between a coordinator and a worker that breaks off, as a worker that died
/// does, when the coordinator sends it its `cut`-th point (from 1): the point is in flight,
/// never answered.
fn breaking(worker: String, cut: usize) -> (String, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let broke = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&broke);
    std::thread::spawn(move || {
        let (mut from, _) = listener.accept().unwrap();
        let mut to = TcpStream::connect(&worker).unwrap();
        // the worker's answers, as they come
        let (mut back_from, mut back_to) = (to.try_clone().unwrap(), from.try_clone().unwrap());
        std::thread::spawn(move || std::io::copy(&mut back_from, &mut back_to));
        let mut points = 0;
        loop {
            let mut length = [0u8; 4];
            if from.read_exact(&mut length).is_err() {
                return;
            }
            let mut body = vec![0u8; u32::from_be_bytes(length) as usize];
            if from.read_exact(&mut body).is_err() {
                return;
            }
            if body.starts_with(b"{\"type\":\"point\"") {
                points += 1;
                if points == cut {
                    flag.store(true, Ordering::Relaxed);
                    let _ = from.shutdown(Shutdown::Both);
                    let _ = to.shutdown(Shutdown::Both);
                    return;
                }
            }
            if to
                .write_all(&length)
                .and_then(|()| to.write_all(&body))
                .is_err()
            {
                return;
            }
        }
    });
    (address, broke)
}

#[test]
fn a_worker_lost_mid_sweep_leaves_its_points_to_the_others_and_the_record_unchanged() {
    for text in [MODES, FDFD] {
        let text = text.replace("points = 6", "points = 12");
        let stop = Stop::new(None);
        let (done, one) = alone(&text, &stop);
        done.unwrap();
        let addresses = workers(3, 1);
        // the first worker's connection breaks at its first point
        let (cut, broke) = breaking(addresses[0].clone(), 1);
        let farm = farm_of(
            &[cut, addresses[1].clone(), addresses[2].clone()],
            FarmOptions::default(),
        );
        let (done, lines) = farmed(&text, &farm, &stop);
        done.unwrap();
        assert!(broke.load(Ordering::Relaxed), "the connection never broke");
        assert_eq!(timeless(&lines), timeless(&one));
    }
}

#[test]
fn a_point_that_breaks_every_worker_fails_with_that_reason() {
    // each of two workers' connections breaks at its first point: the point has lost two
    // workers, all the farm has, and with attempts = 2 it fails rather than waiting for ever
    let addresses = workers(2, 1);
    let cuts: Vec<String> = addresses.iter().map(|a| breaking(a.clone(), 1).0).collect();
    let farm = farm_of(
        &cuts,
        FarmOptions {
            attempts: 2,
            ..FarmOptions::default()
        },
    );
    let (done, _) = farmed(MODES, &farm, &Stop::new(None));
    let e = done.unwrap_err().to_string();
    assert!(
        e.contains("lost 2 workers") || e.contains("no worker left"),
        "{e}"
    );
}

#[test]
fn a_point_that_fails_is_the_sweeps_error_after_the_points_before_it() {
    // silica's data ends before 7 um: the sweep's points past it fail, in one process and
    // farmed alike, after the same points
    let text = MODES
        .replace("to = 1.6", "to = 10.0")
        .replace("points = 6", "points = 4");
    let stop = Stop::new(None);
    let (done, one) = alone(&text, &stop);
    let alone_error = done.unwrap_err().to_string();
    let farm = farm_of(&workers(2, 1), FarmOptions::default());
    let (done, lines) = farmed(&text, &farm, &stop);
    let error = done.unwrap_err().to_string();
    // the same record, but for its last line: each run's failure in its own words
    let failure = |lines: &[String], error: &str| {
        let last: serde_json::Value = serde_json::from_str(lines.last().unwrap()).unwrap();
        assert_eq!(last["type"], "failed");
        assert_eq!(last["error"], error);
    };
    failure(&one, &alone_error);
    failure(&lines, &error);
    assert_eq!(lines[..lines.len() - 1], one[..one.len() - 1]);
    assert!(error.contains(&alone_error), "{error} / {alone_error}");
    assert!(error.starts_with("farm: point "), "{error}");
}

fn structure(name: &str, step_nm: f64) -> String {
    format!(
        r#"
name = "{name}"

[task]
kind = "structure"
stack = "soi_220"
wavelength_um = 1.55
layer = "Si"
x_um = [-3.0, 3.0]
y_um = [-2.0, 2.0]
step_nm = {step_nm}

[[task.rect]]
layer = "Si"
center_um = [0.0, 0.0]
size_um = [6.0, 0.5]
"#
    )
}

#[test]
fn whole_jobs_come_back_in_their_order_whichever_finishes_first() {
    // the first jobs the slowest (the finest grid), so that later ones finish first, on
    // workers of 1, 2 and 4 slots: job k's record is job k's, and the record it has alone
    let jobs: Vec<Job> = (0..24)
        .map(|k| Job::parse(&structure(&format!("job-{k}"), 5.0 + 2.0 * k as f64)).unwrap())
        .collect();
    let mut addresses = workers(1, 1);
    addresses.extend(workers(1, 2));
    addresses.extend(workers(1, 4));
    let farm = farm_of(&addresses, FarmOptions::default());
    let stop = Stop::new(None);
    let records = farm.run_jobs(&jobs, &stop).unwrap();
    assert_eq!(records.len(), jobs.len());
    for (k, record) in records.into_iter().enumerate() {
        let lines = record.unwrap();
        assert!(
            lines[0].contains(&format!("\"job\":\"job-{k}\"")),
            "{k}: {}",
            lines[0]
        );
        let (done, one) = alone(jobs[k].text(), &stop);
        done.unwrap();
        assert_eq!(timeless(&lines), timeless(&one), "job {k}");
    }
}

#[test]
fn a_job_that_fails_is_its_result() {
    let good = Job::parse(&structure("good", 40.0)).unwrap();
    let bad =
        Job::parse(&structure("bad", 40.0).replace("layer = \"Si\"\nx_um", "layer = \"Ge\"\nx_um"))
            .unwrap();
    let farm = farm_of(&workers(2, 1), FarmOptions::default());
    let records = farm
        .run_jobs(&[good.clone(), bad, good], &Stop::new(None))
        .unwrap();
    assert!(records[0].is_ok() && records[2].is_ok());
    let e = records[1].as_ref().unwrap_err().to_string();
    assert!(e.contains("job 1") && e.contains("Ge"), "{e}");
}

#[test]
fn a_task_past_its_timeout_fails_with_that_reason() {
    // a modes job takes far longer than 5 ms
    let job = Job::parse(unswept(MODES)).unwrap();
    let farm = farm_of(
        &workers(1, 1),
        FarmOptions {
            task_timeout: Some(Duration::from_millis(5)),
            ..FarmOptions::default()
        },
    );
    let records = farm.run_jobs(&[job], &Stop::new(None)).unwrap();
    let e = records[0].as_ref().unwrap_err().to_string();
    assert!(e.contains("past its timeout of 0.005 s"), "{e}");
}

/// A farm's sweep, timed: when the run gave it its points, and when it gave them back.
#[derive(Debug)]
struct Timed {
    farm: Box<dyn crate::run::Points>,
    span: Arc<Mutex<Option<(Instant, Instant)>>>,
}

impl crate::run::Points for Timed {
    fn sweep(
        &mut self,
        run: &mut Run,
        stop: &Stop,
        points: std::ops::Range<usize>,
        point: &crate::run::PointFn<'_>,
    ) -> crate::Result<()> {
        let began = Instant::now();
        let done = self.farm.sweep(run, stop, points, point);
        *self.span.lock().unwrap() = Some((began, Instant::now()));
        done
    }
}

/// The job run with its sweep farmed and a stop at `deadline`: its result, its record, how long
/// the farm went on past the deadline, and whether the run's sweep began before it. The farm's
/// time is its sweep's past the deadline (past its start, if the run's own setup ran past the
/// deadline) and its letting the workers go; the run's own steps, here on the coordinator,
/// aren't the farm's.
fn farmed_until(
    text: &str,
    farm: &Farm,
    deadline: Duration,
) -> (crate::Result<()>, Vec<String>, Duration, bool) {
    let root = TempDir::new("until");
    let job = Job::parse(text).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    // (no later than the stop's own deadline)
    let at = Instant::now() + deadline;
    let stop = Stop::new(Some(deadline));
    farm.farm_sweep(&job, &mut run, &stop);
    let span = Arc::new(Mutex::new(None));
    let sweep = run.take_points().unwrap();
    run.set_points(Box::new(Timed {
        farm: sweep,
        span: Arc::clone(&span),
    }));
    let done = execute(&job, &mut run, &stop);
    let lines = lines_of(&run);
    // a sweep the run never came to lets its workers go when the run drops it
    let dropped = Instant::now();
    drop(run);
    let mut late = dropped.elapsed();
    let span = *span.lock().unwrap();
    if let Some((began, ended)) = span {
        late += ended.saturating_duration_since(began.max(at));
    }
    (done, lines, late, span.is_some_and(|(began, _)| began < at))
}

#[test]
fn a_farm_stops_at_its_deadline_with_the_points_done_recorded_in_order() {
    let text = MODES.replace("points = 6", "points = 40");
    let started = Instant::now();
    let (done, all) = alone(&text, &Stop::new(None));
    done.unwrap();
    let whole = started.elapsed();
    let farm = farm_of(&workers(2, 1), FarmOptions::default());
    // A deadline of a third of the sweep alone, which two workers can't meet; but on a machine
    // that other work slowed while the sweep ran alone, and spares while it is farmed, they
    // can: then the farm tries again at half the deadline. And on a machine that other work
    // slows while the sweep is farmed, the run's own setup (its modes before the sweep, solved
    // here, as in a run alone) can run past the deadline, where the farm has nothing to stop:
    // then it tries again at twice the deadline. In every try the farm must stop soon after the
    // deadline (the run's own steps end at a stop as in a run alone, between its solve's steps,
    // one of which, an LU factorization, other work on rayon's threads can hold up for
    // seconds), and a try that finished must have recorded the whole sweep. A farm that
    // ignored its deadline goes on past it, or finishes every try, and fails the fifth.
    let mut deadline = whole / 3;
    let mut finished = 0;
    for attempt in 1..=10 {
        let (done, lines, late, in_sweep) = farmed_until(&text, &farm, deadline);
        done.unwrap();
        // it stopped soon after its deadline, not when the workers were done
        assert!(
            late < Duration::from_secs(2),
            "try {attempt}: the farm went on {late:?} past its deadline of {deadline:?}",
        );
        // the points done, in order
        let (last, before) = lines.split_last().unwrap();
        assert_eq!(before, &all[..before.len()]);
        if before.len() == all.len() - 1 {
            finished += 1;
            assert!(
                finished < 5,
                "it didn't stop, at a deadline of {deadline:?}"
            );
            println!("try {attempt}: the whole sweep done by a deadline of {deadline:?}");
            deadline /= 2;
            continue;
        }
        // cut short, and it says why
        assert!(last.contains("\"stopped\":\"timeout\""), "{last}");
        if in_sweep {
            return;
        }
        println!("try {attempt}: the deadline of {deadline:?} passed before the sweep");
        deadline *= 2;
    }
    panic!("in no try did the deadline pass in the sweep");
}

/// A connection between a coordinator and a worker that passes everything on but the worker's
/// answers to points, which it keeps: the points stay in flight. It closes after `hold`. The
/// count is of the answers kept.
fn holding(worker: String, hold: Duration) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let held = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&held);
    std::thread::spawn(move || {
        let (from, _) = listener.accept().unwrap();
        let mut back = TcpStream::connect(&worker).unwrap();
        // the coordinator's messages, as they come
        let (mut forth_from, mut forth_to) = (from.try_clone().unwrap(), back.try_clone().unwrap());
        std::thread::spawn(move || std::io::copy(&mut forth_from, &mut forth_to));
        let (end_from, end_back) = (from.try_clone().unwrap(), back.try_clone().unwrap());
        std::thread::spawn(move || {
            std::thread::sleep(hold);
            let _ = end_from.shutdown(Shutdown::Both);
            let _ = end_back.shutdown(Shutdown::Both);
        });
        let mut to = from;
        loop {
            let mut length = [0u8; 4];
            if back.read_exact(&mut length).is_err() {
                return;
            }
            let mut body = vec![0u8; u32::from_be_bytes(length) as usize];
            if back.read_exact(&mut body).is_err() {
                return;
            }
            if [
                &b"{\"type\":\"events\""[..],
                b"{\"type\":\"stopped\"",
                b"{\"type\":\"failed\"",
            ]
            .iter()
            .any(|p| body.starts_with(p))
            {
                count.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            if to
                .write_all(&length)
                .and_then(|()| to.write_all(&body))
                .is_err()
            {
                return;
            }
        }
    });
    (address, held)
}

#[test]
fn a_farm_at_its_stop_leaves_the_points_in_flight() {
    // a worker whose answers to points never reach the farm: when the run is stopped, the farm
    // must let the point it has in flight go and end the run, not wait for its answer (on
    // Windows it waited: the time a read waits for was set on another handle of the socket than
    // the one read, and on Windows each has its own; #388)
    let (address, held) = holding(workers(1, 1).remove(0), Duration::from_secs(60));
    let farm = farm_of(&[address], FarmOptions::default());
    let stop = Stop::new(None);
    let requested = Arc::new(Mutex::new(None));
    {
        let (stop, held, requested) = (stop.clone(), Arc::clone(&held), Arc::clone(&requested));
        // the stop once an answer was kept: the sweep is under way, its next point in flight
        std::thread::spawn(move || {
            while held.load(Ordering::Relaxed) == 0 {
                std::thread::sleep(Duration::from_millis(10));
            }
            *requested.lock().unwrap() = Some(Instant::now());
            stop.request();
        });
    }
    let (done, lines) = farmed(MODES, &farm, &stop);
    let late = requested.lock().unwrap().expect("no answer kept").elapsed();
    done.unwrap();
    assert!(
        late < Duration::from_secs(2),
        "the run ended {late:?} after its stop"
    );
    assert_eq!(points_in(&lines), 0);
    let last = lines.last().unwrap();
    assert!(last.contains("\"stopped\":\"requested\""), "{last}");
}

#[test]
fn a_worker_refuses_a_wrong_token() {
    let addresses = workers(1, 1);
    let mut farm = Farm::new(FarmOptions {
        token: Some("not the token".into()),
        ..FarmOptions::default()
    });
    farm.add_remote(&addresses[0]);
    let job = Job::parse(&structure("refused", 40.0)).unwrap();
    let e = farm
        .run_jobs(&[job], &Stop::new(None))
        .unwrap_err()
        .to_string();
    assert!(e.contains("refused: wrong token"), "{e}");
}

#[test]
fn a_worker_beyond_this_machine_needs_a_token() {
    let e = Server::bind("0.0.0.0:0", ServeOptions::default())
        .unwrap_err()
        .to_string();
    assert!(e.contains("needs a token"), "{e}");
    assert!(
        Server::bind(
            "0.0.0.0:0",
            ServeOptions {
                token: Some(TOKEN.into()),
                ..ServeOptions::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn a_worker_refuses_what_isnt_a_hello_and_an_overlong_one() {
    let address = workers(1, 1).remove(0);
    for frame in [
        // JSON, but no message of the protocol
        {
            let body = br#"{"type":"shell","command":"rm"}"#;
            let mut f = (body.len() as u32).to_be_bytes().to_vec();
            f.extend_from_slice(body);
            f
        },
        // a length past what a hello may have, with nothing after it
        (1u32 << 20).to_be_bytes().to_vec(),
        // a hello of another version
        {
            let body = br#"{"type":"hello","protocol":1,"photonoxide":"0.0.1","token":"a token for the tests"}"#;
            let mut f = (body.len() as u32).to_be_bytes().to_vec();
            f.extend_from_slice(body);
            f
        },
    ] {
        let mut stream = TcpStream::connect(&address).unwrap();
        stream.write_all(&frame).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        let mut reader = protocol::Reader::new(stream);
        match reader.wait().unwrap() {
            protocol::Message::Refused { reason } => assert!(!reason.is_empty()),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn a_worker_refuses_a_connection_that_says_nothing_within_the_handshake() {
    // (a worker serves one coordinator at a time: one that never says hello would keep it for
    // ever, as it did on Windows, where the hello's time was set on another handle of the
    // socket than the one read)
    let address = workers(1, 1).remove(0);
    let stream = TcpStream::connect(&address).unwrap();
    stream
        .set_read_timeout(Some(HANDSHAKE + Duration::from_secs(10)))
        .unwrap();
    let started = Instant::now();
    match protocol::Reader::new(stream).wait().unwrap() {
        protocol::Message::Refused { reason } => assert!(reason.contains("no message"), "{reason}"),
        other => panic!("{other:?}"),
    }
    assert!(started.elapsed() >= HANDSHAKE, "{:?}", started.elapsed());
}

#[test]
fn tokens_compare_whole() {
    assert!(same_token("abc", "abc"));
    assert!(!same_token("abc", "abd"));
    assert!(!same_token("abc", "abcd"));
    assert!(!same_token("", "a"));
    assert!(same_token("", ""));
}

#[test]
fn a_record_line_is_written_as_it_is_and_counted_in_the_digest() {
    let root = TempDir::new("line");
    let job = Job::parse(&structure("line", 40.0)).unwrap();
    let mut run = Run::create(&root.0, &job).unwrap();
    let mut memory = Run::in_memory();
    let mut discard = Run::discarding();
    let event = crate::job::Event::Started {
        job: "line".into(),
        kind: "structure".into(),
    };
    run.record(&event).unwrap();
    memory
        .record_line(&serde_json::to_string(&event).unwrap())
        .unwrap();
    discard.record(&event).unwrap();
    assert_eq!(run.digest(), memory.digest());
    assert_eq!(run.digest(), discard.digest());
    assert_eq!(lines_of(&run), memory.lines());
    assert!(discard.lines().is_empty());
    assert!(run.record_line("{}\n{}").is_err());
}

/// A generation of genoxide's CMA-ES evaluated on the farm, through genoxide's `Batch`: a
/// strip's width chosen for its TE mode's effective index, each candidate a modes job. The
/// same run on 1 and 3 workers, and with the candidates solved here, finds the same best to
/// the last bit.
#[test]
fn a_genoxide_population_is_evaluated_on_the_farm() {
    use genoxide::prelude::{Batch, Cmaes, Engine, Real, Reals};
    const TARGET: f64 = 2.3;
    let job = |width: f64| {
        let text = unswept(MODES)
            .replace(
                "size_um = [10.0, 0.5]",
                &format!("size_um = [10.0, {width:?}]"),
            )
            .replace("step_nm = 25.0", "step_nm = 50.0")
            .replace("modes = 2", "modes = 1");
        Job::parse(&text).unwrap()
    };
    // the first mode's effective index, from its record
    let index = |lines: &[String]| -> f64 {
        let mode = lines
            .iter()
            .find(|l| l.starts_with("{\"type\":\"mode\""))
            .expect("a mode");
        let v: serde_json::Value = serde_json::from_str(mode).unwrap();
        v["effective_index"][0].as_f64().unwrap()
    };
    let optimize = |evaluate: &(dyn Fn(&[Job]) -> Vec<Vec<String>> + Sync)| {
        let cmaes = Cmaes::builder(Real::new([0.3..=0.7]).unwrap())
            .initial_mean(Reals::from(vec![0.4]))
            .initial_step(0.05)
            .minimize()
            .seed(7)
            .build()
            .unwrap();
        let fitness = Batch(|genomes: &[&Reals]| {
            let jobs: Vec<Job> = genomes.iter().map(|g| job(g[0])).collect();
            evaluate(&jobs)
                .iter()
                .map(|lines| (index(lines) - TARGET).abs())
                .collect::<Vec<f64>>()
        });
        let outcome = Engine::new(cmaes, fitness)
            .stop_when(genoxide::prelude::Stop::generations(3))
            .run()
            .unwrap();
        (
            outcome.best().genome()[0].to_bits(),
            outcome.best_fitness().score().unwrap().to_bits(),
            outcome.evaluations(),
        )
    };
    let here = optimize(&|jobs: &[Job]| {
        jobs.iter()
            .map(|j| {
                let mut run = Run::in_memory();
                execute(j, &mut run, &Stop::new(None)).unwrap();
                run.lines().to_vec()
            })
            .collect()
    });
    assert!(here.2 >= 12, "{here:?}");
    for count in [1, 3] {
        let farm = farm_of(&workers(count, 1), FarmOptions::default());
        let farmed = optimize(&|jobs: &[Job]| {
            farm.run_jobs(jobs, &Stop::new(None))
                .unwrap()
                .into_iter()
                .map(Result::unwrap)
                .collect()
        });
        assert_eq!(farmed, here, "{count} workers");
    }
}
