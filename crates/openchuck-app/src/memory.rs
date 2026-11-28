// SPDX-License-Identifier: AGPL-3.0-or-later
//! The computer's physical memory, and the share of it one toolpath job may
//! use (issue #847, the owner's answers): "Memory a toolpath job may use",
//! a percentage of the computer's memory, 10 to 90 % in steps of 5, half by
//! default. It is the host's own limit: a job that would pass it is stopped
//! with an error and nothing is made. It is not a guarantee that the system
//! will not stop the program when the computer runs short of memory. It
//! never changes what a job that finishes makes (GC-07), only whether it
//! finishes, so a project can finish on one computer and stop on another
//! with less memory. Playback's checkpoints have their own limit (decision
//! record 0322).

/// Bytes in a GB of 2^30 bytes.
const GIB: u64 = 1024 * 1024 * 1024;

/// The share of the computer's memory a toolpath job may use, percent: the
/// least, the default, the most and the step.
pub const JOB_MEMORY_PERCENT: (u8, u8, u8, u8) = (10, 50, 90, 5);

/// Above this share Settings warns that the system and other programs need
/// memory too.
pub const JOB_MEMORY_NOTE_ABOVE: u8 = 75;

/// Where the memory limit of the jobs was set, for the words of a stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOrigin {
    /// The app's Settings, "Memory a toolpath job may use".
    Settings,
    /// The command line's `--job-memory`.
    CommandLine,
}

/// The memory limit of the toolpath jobs: its share of the computer's
/// memory, in bytes, and where it was set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobMemory {
    /// The share of the computer's memory, percent.
    pub percent: u8,
    /// The limit, bytes.
    pub bytes: u64,
    /// Where it was set.
    pub origin: MemoryOrigin,
}

impl JobMemory {
    /// The limit for `percent` of a computer with `physical` bytes, set at
    /// `origin`; `None` where the memory cannot be read (no limit then).
    pub fn of(percent: u8, physical: Option<u64>, origin: MemoryOrigin) -> Option<Self> {
        let percent = clamp_percent(percent);
        let physical = physical.filter(|&bytes| bytes > 0)?;
        let bytes = u64::try_from(u128::from(physical) * u128::from(percent) / 100).ok()?;
        Some(Self {
            percent,
            bytes,
            origin,
        })
    }
}

/// `percent` brought into the range, to the nearest step.
pub fn clamp_percent(percent: u8) -> u8 {
    let (least, _, most, step) = JOB_MEMORY_PERCENT;
    let stepped = (u16::from(percent) + u16::from(step) / 2) / u16::from(step) * u16::from(step);
    u8::try_from(stepped).unwrap_or(most).clamp(least, most)
}

/// A size in GB of 2^30 bytes, as the memory texts give it: whole GB from
/// 10 GB, one decimal below.
pub fn size_words(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let gb = bytes as f64 / GIB as f64;
    if gb >= 10.0 {
        format!("{gb:.0} GB")
    } else {
        format!("{gb:.1} GB")
    }
}

/// The note under Adaptive work limit (issue #847): where the most memory
/// the operation could need at `work` geometry queries is more than a
/// toolpath job may use, what each is and what then happens. `None` where
/// it fits, or where there is no limit. A note, not a refusal: most parts
/// never come near the most.
pub fn work_limit_note(work: u64, memory: Option<JobMemory>) -> Option<String> {
    let memory = memory?;
    let most = openchuck_path::adaptive::most_memory(work);
    (most > memory.bytes).then(|| {
        let r#where = match memory.origin {
            MemoryOrigin::Settings => "Settings",
            MemoryOrigin::CommandLine => "--job-memory",
        };
        format!(
            "At this limit the operation could need up to {}; toolpath jobs may use {} on \
             this computer ({where}). If it needs more, it is stopped with an error.",
            size_words(most),
            size_words(memory.bytes),
        )
    })
}

/// The computer's memory in whole GB of 2^30 bytes, to the nearest, as
/// Settings names it.
pub fn gigabytes(physical: Option<u64>) -> Option<u64> {
    physical.map(|bytes| bytes.saturating_add(GIB / 2) / GIB)
}

/// The physical memory of this computer, in bytes, if it can be read.
pub fn physical() -> Option<u64> {
    platform()
}

/// `MemTotal` of Linux's `/proc/meminfo`, in bytes.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn from_meminfo(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let mut words = line.split_whitespace().skip(1);
    let number: u64 = words.next()?.parse().ok()?;
    match words.next() {
        Some("kB") => number.checked_mul(1024),
        _ => None,
    }
}

/// The output of `sysctl -n hw.memsize` (macOS), in bytes.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn from_sysctl(text: &str) -> Option<u64> {
    text.trim().parse().ok().filter(|&bytes| bytes > 0)
}

#[cfg(target_os = "linux")]
fn platform() -> Option<u64> {
    from_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

#[cfg(target_os = "macos")]
fn platform() -> Option<u64> {
    let output = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    from_sysctl(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn platform() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: u32::try_from(std::mem::size_of::<MEMORYSTATUSEX>()).ok()?,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };
    // SAFETY: the pointer is to a live MEMORYSTATUSEX whose dwLength is set
    // to its size, as the call requires; it writes only into that struct.
    let done = unsafe { GlobalMemoryStatusEx(&raw mut status) };
    (done != 0 && status.ullTotalPhys > 0).then_some(status.ullTotalPhys)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn platform() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn req_nfr_07_the_physical_memory_is_read_from_the_platforms_text() {
        let meminfo = "MemTotal:       16303740 kB\nMemFree:        15000000 kB\n";
        assert_eq!(from_meminfo(meminfo), Some(16_303_740 * 1024));
        assert_eq!(from_meminfo("MemFree: 1 kB\n"), None);
        assert_eq!(from_meminfo("MemTotal: lots kB\n"), None);
        assert_eq!(from_meminfo("MemTotal: 12\n"), None);
        assert_eq!(from_sysctl("34359738368\n"), Some(32 * GIB));
        assert_eq!(from_sysctl("0\n"), None);
        assert_eq!(from_sysctl("sysctl: unknown oid\n"), None);
        // This computer's memory, when it can be read, is above zero.
        assert!(physical().is_none_or(|bytes| bytes > 0));
    }

    /// Issue #847 (the owner's Q6 and Q7 answers): half the computer's
    /// memory by default, 10 to 90 % in steps of 5, and no limit where the
    /// memory cannot be read.
    #[test]
    fn req_nfr_07_a_job_may_use_its_share_of_the_computers_memory() {
        let half = JobMemory::of(50, Some(64 * GIB), MemoryOrigin::Settings).unwrap();
        assert_eq!(half.bytes, 32 * GIB);
        assert_eq!(half.percent, 50);
        assert_eq!(JOB_MEMORY_PERCENT.1, 50);
        assert_eq!(clamp_percent(0), 10);
        assert_eq!(clamp_percent(5), 10);
        assert_eq!(clamp_percent(52), 50);
        assert_eq!(clamp_percent(53), 55);
        assert_eq!(clamp_percent(95), 90);
        assert_eq!(clamp_percent(255), 90);
        assert_eq!(
            JobMemory::of(200, Some(10 * GIB), MemoryOrigin::CommandLine)
                .unwrap()
                .bytes,
            9 * GIB
        );
        assert_eq!(JobMemory::of(50, None, MemoryOrigin::Settings), None);
        assert_eq!(JobMemory::of(50, Some(0), MemoryOrigin::Settings), None);
        assert_eq!(size_words(32 * GIB), "32 GB");
        assert_eq!(size_words(31 * GIB + GIB / 3), "31 GB");
        assert_eq!(size_words(GIB * 3 / 2), "1.5 GB");
        assert_eq!(gigabytes(Some(16_303_740 * 1024)), Some(16));
    }
}
