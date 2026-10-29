//! The CPU's cores, caches and NUMA nodes (#564), for the performance advice: how many physical
//! cores there are, how many of them are performance and how many efficiency cores on a hybrid
//! CPU, the size of one cache at each level, and the NUMA nodes.
//!
//! Read where the system keeps them, with no library: Windows's `GetLogicalProcessorInformationEx`,
//! Linux's `/sys/devices/system`, macOS's `sysctl`. What a system doesn't say is left out
//! (`None`, or 0): Apple's chips have no L3 cache, a Linux in a container may hide `/sys`.

use serde::Serialize;

/// The shape of the CPU.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Topology {
    /// Physical cores; 0 where the system doesn't say.
    pub cores: usize,
    /// On a hybrid CPU, how many of the cores are the fast kind (P-cores) and how many the
    /// slow, frugal kind (E-cores); both 0 on a CPU whose cores are all alike.
    pub performance_cores: usize,
    pub efficiency_cores: usize,
    /// One cache of each level, in bytes: the L1 data cache, the L2 and the L3 (the largest of
    /// its level, where a CPU has several sizes).
    pub l1d: Option<u64>,
    pub l2: Option<u64>,
    pub l3: Option<u64>,
    /// NUMA nodes: 1 on a machine with one memory, more on a multi-socket or chiplet server.
    pub numa_nodes: usize,
}

/// What the system listed, before it is summed up.
#[derive(Debug, Default)]
struct Raw {
    /// One entry per physical core: its class, higher for a faster kind of core.
    cores: Vec<u32>,
    /// One entry per cache: its level, whether it holds instructions only, its size in bytes.
    caches: Vec<(u8, bool, u64)>,
    nodes: usize,
}

/// This machine's.
pub fn here() -> Topology {
    summarise(&raw_here())
}

fn summarise(raw: &Raw) -> Topology {
    let fastest = raw.cores.iter().copied().max();
    // hybrid: cores of more than one class; the fastest class is the P-cores
    let hybrid = raw.cores.iter().any(|c| Some(*c) != fastest);
    let performance = if hybrid {
        raw.cores.iter().filter(|c| Some(**c) == fastest).count()
    } else {
        0
    };
    let largest = |level: u8| {
        raw.caches
            .iter()
            .filter(|(l, instruction, _)| *l == level && !instruction)
            .map(|(_, _, bytes)| *bytes)
            .max()
    };
    Topology {
        cores: raw.cores.len(),
        performance_cores: performance,
        efficiency_cores: if hybrid {
            raw.cores.len() - performance
        } else {
            0
        },
        l1d: largest(1),
        l2: largest(2),
        l3: largest(3),
        numa_nodes: raw.nodes,
    }
}

/// The records `GetLogicalProcessorInformationEx(RelationAll)` fills a buffer with: a core's
/// efficiency class (higher is faster, 0 on a CPU of one kind), a cache's level, type and size,
/// and a NUMA node's number.
#[cfg(any(windows, test))]
fn parse_windows(buffer: &[u8]) -> Raw {
    const CORE: u32 = 0;
    const NUMA_NODE: u32 = 1;
    const CACHE: u32 = 2;
    const NUMA_NODE_EX: u32 = 6;
    const INSTRUCTION: u32 = 1;
    let word = |record: &[u8], at: usize| {
        record
            .get(at..at + 4)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let mut raw = Raw::default();
    let mut nodes = std::collections::BTreeSet::new();
    let mut at = 0;
    while at + 8 <= buffer.len() {
        let kind = word(buffer, at);
        let size = word(buffer, at + 4) as usize;
        if size < 8 || at + size > buffer.len() {
            break;
        }
        let record = &buffer[at..at + size];
        match kind {
            CORE if size > 9 => raw.cores.push(u32::from(record[9])),
            NUMA_NODE | NUMA_NODE_EX => {
                nodes.insert(word(record, 8));
            }
            CACHE if size >= 20 => raw.caches.push((
                record[8],
                word(record, 16) == INSTRUCTION,
                u64::from(word(record, 12)),
            )),
            _ => {}
        }
        at += size;
    }
    raw.nodes = nodes.len();
    raw
}

#[cfg(windows)]
fn raw_here() -> Raw {
    unsafe extern "system" {
        fn GetLogicalProcessorInformationEx(
            relationship: u32,
            buffer: *mut u8,
            length: *mut u32,
        ) -> i32;
    }
    const ALL: u32 = 0xffff;
    let mut length = 0u32;
    // the first call fails and says how large a buffer is needed
    unsafe { GetLogicalProcessorInformationEx(ALL, std::ptr::null_mut(), &raw mut length) };
    if length == 0 {
        return Raw::default();
    }
    let mut buffer = vec![0u8; length as usize];
    let ok = unsafe { GetLogicalProcessorInformationEx(ALL, buffer.as_mut_ptr(), &raw mut length) };
    if ok == 0 {
        return Raw::default();
    }
    buffer.truncate(length as usize);
    parse_windows(&buffer)
}

/// A cache size as sysfs writes it: `48K`, `2048K`, `32M`, or plain bytes.
#[cfg(any(target_os = "linux", test))]
fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let (digits, unit) = match text.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((i, _)) => text.split_at(i),
        None => (text, ""),
    };
    let n: u64 = digits.parse().ok()?;
    match unit {
        "" => Some(n),
        "K" | "k" => Some(n << 10),
        "M" | "m" => Some(n << 20),
        "G" | "g" => Some(n << 30),
        _ => None,
    }
}

/// The CPU numbers in a sysfs list: `0-7,16-23`.
#[cfg(any(target_os = "linux", test))]
fn parse_list(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for part in text.trim().split(',').filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.trim().parse::<usize>(), b.trim().parse::<usize>()) {
                    out.extend(a..=b);
                }
            }
            None => out.extend(part.trim().parse::<usize>()),
        }
    }
    out
}

/// One logical CPU's place, as Linux lists it.
#[cfg(any(target_os = "linux", test))]
struct Logical {
    package: u32,
    core: u32,
    /// Its core's kind: the CPU's capacity where Linux gives one (Arm's big.LITTLE and Intel's
    /// hybrid both do on recent kernels), else 1 for Intel's P-cores and 0 for its E-cores.
    class: u32,
}

/// The physical cores among `logical` CPUs: one per package and core number, with the class of
/// its first thread.
#[cfg(any(target_os = "linux", test))]
fn linux_cores(logical: &[Logical]) -> Vec<u32> {
    let mut seen = std::collections::BTreeMap::new();
    for l in logical {
        seen.entry((l.package, l.core)).or_insert(l.class);
    }
    seen.into_values().collect()
}

#[cfg(target_os = "linux")]
fn raw_here() -> Raw {
    use std::fs::read_to_string;
    let cpu = "/sys/devices/system/cpu";
    let read = |path: String| read_to_string(path).ok();
    // Intel hybrid: the CPUs that are E-cores
    let atoms = read("/sys/devices/cpu_atom/cpus".to_owned())
        .map(|t| parse_list(&t))
        .unwrap_or_default();
    let mut logical = Vec::new();
    let mut caches = Vec::new();
    for n in 0.. {
        let dir = format!("{cpu}/cpu{n}");
        if !std::path::Path::new(&dir).is_dir() {
            // numbers can skip while a CPU is off: stop at a long gap
            if n > 4096 {
                break;
            }
            continue;
        }
        let number = |name: &str| read(format!("{dir}/topology/{name}"))?.trim().parse().ok();
        let (Some(package), Some(core)) = (number("physical_package_id"), number("core_id")) else {
            continue;
        };
        let capacity = read(format!("{dir}/cpu_capacity")).and_then(|t| t.trim().parse().ok());
        logical.push(Logical {
            package,
            core,
            class: capacity.unwrap_or_else(|| u32::from(!atoms.contains(&n))),
        });
        // every CPU lists its caches, shared ones too: the largest of a level is kept
        for index in 0..8 {
            let cache = format!("{dir}/cache/index{index}");
            let (Some(level), Some(kind), Some(size)) = (
                read(format!("{cache}/level")),
                read(format!("{cache}/type")),
                read(format!("{cache}/size")).and_then(|t| parse_size(&t)),
            ) else {
                break;
            };
            if let Ok(level) = level.trim().parse() {
                caches.push((level, kind.trim() == "Instruction", size));
            }
        }
    }
    let nodes = std::fs::read_dir("/sys/devices/system/node")
        .map(|d| {
            d.flatten()
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.strip_prefix("node")
                        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                })
                .count()
        })
        .unwrap_or(0);
    Raw {
        cores: linux_cores(&logical),
        caches,
        nodes,
    }
}

#[cfg(target_os = "macos")]
fn raw_here() -> Raw {
    let sysctl = |name: &str| -> Option<u64> {
        let out = std::process::Command::new("sysctl")
            .args(["-n", name])
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        String::from_utf8_lossy(&out.stdout).trim().parse().ok()
    };
    let mut cores = Vec::new();
    // Apple silicon: level 0 is the performance cores, level 1 the efficiency cores
    match (
        sysctl("hw.perflevel0.physicalcpu"),
        sysctl("hw.perflevel1.physicalcpu"),
    ) {
        (Some(p), Some(e)) => {
            cores.extend(std::iter::repeat_n(1, p as usize));
            cores.extend(std::iter::repeat_n(0, e as usize));
        }
        _ => cores.extend(std::iter::repeat_n(
            0,
            sysctl("hw.physicalcpu").unwrap_or(0) as usize,
        )),
    }
    let mut caches = Vec::new();
    for (level, name) in [
        (1, "hw.l1dcachesize"),
        (2, "hw.l2cachesize"),
        (3, "hw.l3cachesize"),
    ] {
        if let Some(bytes) = sysctl(name).filter(|b| *b > 0) {
            caches.push((level, false, bytes));
        }
    }
    Raw {
        cores,
        caches,
        // a Mac has one memory
        nodes: 1,
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn raw_here() -> Raw {
    Raw::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kind: u32, body: &[u8]) -> Vec<u8> {
        let mut r = kind.to_le_bytes().to_vec();
        r.extend(((8 + body.len()) as u32).to_le_bytes());
        r.extend(body);
        r
    }

    fn core(class: u8) -> Vec<u8> {
        // Flags, EfficiencyClass, then the rest of PROCESSOR_RELATIONSHIP
        let mut body = vec![0, class];
        body.extend([0; 22]);
        record(0, &body)
    }

    fn cache(level: u8, kind: u32, size: u32) -> Vec<u8> {
        let mut body = vec![level, 8, 64, 0];
        body.extend(size.to_le_bytes());
        body.extend(kind.to_le_bytes());
        body.extend([0; 30]);
        record(2, &body)
    }

    fn node(number: u32) -> Vec<u8> {
        let mut body = number.to_le_bytes().to_vec();
        body.extend([0; 20]);
        record(1, &body)
    }

    #[test]
    fn windows_lists_a_hybrid_cpu() {
        // 2 P-cores (class 1), 4 E-cores (class 0); L1 data and instruction, L2, L3; one node
        let mut buffer = Vec::new();
        for class in [1, 1, 0, 0, 0, 0] {
            buffer.extend(core(class));
        }
        buffer.extend(cache(1, 2, 48 << 10));
        buffer.extend(cache(1, 1, 32 << 10));
        buffer.extend(cache(1, 2, 32 << 10));
        buffer.extend(cache(2, 0, 2 << 20));
        buffer.extend(cache(3, 0, 30 << 20));
        buffer.extend(node(0));
        let t = summarise(&parse_windows(&buffer));
        assert_eq!(
            t,
            Topology {
                cores: 6,
                performance_cores: 2,
                efficiency_cores: 4,
                l1d: Some(48 << 10),
                l2: Some(2 << 20),
                l3: Some(30 << 20),
                numa_nodes: 1,
            }
        );
    }

    #[test]
    fn a_cpu_of_one_kind_is_not_hybrid_and_nodes_count_once() {
        let mut buffer = Vec::new();
        for _ in 0..8 {
            buffer.extend(core(0));
        }
        buffer.extend(node(0));
        buffer.extend(node(1));
        buffer.extend(record(6, &[1, 0, 0, 0, 0, 0, 0, 0]));
        let t = summarise(&parse_windows(&buffer));
        assert_eq!(
            (t.cores, t.performance_cores, t.efficiency_cores),
            (8, 0, 0)
        );
        assert_eq!(t.numa_nodes, 2);
        assert_eq!((t.l1d, t.l2, t.l3), (None, None, None));
    }

    #[test]
    fn a_truncated_buffer_is_read_as_far_as_it_goes() {
        let mut buffer = core(0);
        buffer.extend(&cache(2, 0, 1 << 20)[..10]);
        assert_eq!(summarise(&parse_windows(&buffer)).cores, 1);
        assert_eq!(summarise(&parse_windows(&[])), Topology::default());
    }

    #[test]
    fn linux_sizes_and_lists() {
        assert_eq!(parse_size("48K\n"), Some(48 << 10));
        assert_eq!(parse_size("32M"), Some(32 << 20));
        assert_eq!(parse_size("65536"), Some(65536));
        assert_eq!(parse_size("big"), None);
        assert_eq!(parse_list("0-3,8,10-11\n"), [0, 1, 2, 3, 8, 10, 11]);
        assert!(parse_list("").is_empty());
    }

    #[test]
    fn linux_counts_a_core_once_for_its_threads() {
        // 2 P-cores with 2 threads each (class 1024), 2 E-cores with 1 (class 600)
        let l = |core, class| Logical {
            package: 0,
            core,
            class,
        };
        let logical = [
            l(0, 1024),
            l(0, 1024),
            l(1, 1024),
            l(1, 1024),
            l(8, 600),
            l(9, 600),
        ];
        let t = summarise(&Raw {
            cores: linux_cores(&logical),
            ..Raw::default()
        });
        assert_eq!(
            (t.cores, t.performance_cores, t.efficiency_cores),
            (4, 2, 2)
        );
    }

    #[test]
    fn this_machine_has_cores() {
        let t = here();
        assert!(t.cores >= 1 || cfg!(not(any(windows, target_os = "linux", target_os = "macos"))));
        assert!(t.performance_cores + t.efficiency_cores <= t.cores);
    }
}
