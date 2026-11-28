// SPDX-License-Identifier: AGPL-3.0-or-later
//! The computer's physical memory, and the memory playback's checkpoints may
//! use on it: the user's setting, capped at a quarter of the physical memory
//! (decision record 0322 item 5; the owner's decision on OI-1315). The
//! budget is a ceiling: one above what the computer can hold would make the
//! operating system page to disk or stop the program, which it cannot
//! report as a clear error (NFR-07). The memory also sets whether playback
//! information is made with toolpath generation by default (decision record
//! 0333).

/// Bytes in a MB of 2^20 bytes.
const MIB: u64 = 1024 * 1024;

/// The memory playback may use, in bytes, for a setting of `setting_mb` MB
/// on a computer with `physical` bytes (unknown: the setting alone), and
/// the cap in MB when it is below the setting.
pub(crate) fn playback_memory(setting_mb: u32, physical: Option<u64>) -> (usize, Option<u32>) {
    let wanted = u64::from(setting_mb).saturating_mul(MIB);
    let cap = physical.map(|bytes| bytes / 4);
    match cap {
        Some(cap) if cap < wanted => {
            let cap_mb = u32::try_from(cap / MIB).unwrap_or(u32::MAX);
            (usize::try_from(cap).unwrap_or(usize::MAX), Some(cap_mb))
        }
        _ => (usize::try_from(wanted).unwrap_or(usize::MAX), None),
    }
}

/// Whether "Generate playback information with toolpath generation" is on
/// when the user has not chosen, on a computer with `physical` bytes
/// (decision record 0333, the owner's decision): on with 16 GB or more, off
/// below and when the memory cannot be read. A computer sold as 16 GB
/// reports a little less (memory the system keeps for itself), so 15 GB
/// counts.
pub(crate) fn playback_with_generation_default(physical: Option<u64>) -> bool {
    physical.is_some_and(|bytes| bytes >= 15 * 1024 * MIB)
}

/// The computer's memory in whole GB of 2^30 bytes, to the nearest, as
/// Settings names it.
pub(crate) fn gigabytes(physical: Option<u64>) -> Option<u64> {
    openchuck_app::memory::gigabytes(physical)
}

/// The physical memory of this computer, in bytes, if it can be read.
pub(crate) fn physical() -> Option<u64> {
    openchuck_app::memory::physical()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * MIB;

    #[test]
    fn req_nfr_07_playback_memory_is_capped_at_a_quarter_of_the_computer() {
        // The owner's laptop, 32 GB: the default 1,536 MB is not capped.
        assert_eq!(
            playback_memory(1_536, Some(32 * GIB)),
            (1_536 * 1024 * 1024, None)
        );
        // 8,192 MB is exactly a quarter of 32 GB: not capped.
        assert_eq!(playback_memory(8_192, Some(32 * GIB)).1, None);
        // On 4 GB, 1,536 MB is capped at 1,024 MB, and Settings says so.
        assert_eq!(
            playback_memory(1_536, Some(4 * GIB)),
            (1_024 * 1024 * 1024, Some(1_024))
        );
        // Memory that cannot be read leaves the setting as it is.
        assert_eq!(playback_memory(4_096, None), (4_096 * 1024 * 1024, None));
        // An odd size is rounded down, never up.
        let (bytes, cap) = playback_memory(8_192, Some(6 * GIB + 3));
        assert_eq!(
            bytes as u64,
            6 * GIB / 4,
            "3 bytes over a multiple of 4 are dropped"
        );
        assert_eq!(cap, Some(1_536));
    }

    /// The owner's decision on record 0333: playback information is made
    /// with toolpath generation by default on a computer of 16 GB or more,
    /// not below, and not when the memory cannot be read; a computer sold as
    /// 16 GB reports a little less, and still counts.
    #[test]
    fn req_sim_04_req_nfr_07_playback_with_generation_is_on_by_default_from_16_gb() {
        let sixteen = 16_303_740 * 1024;
        for (physical, on, gb) in [
            (Some(32 * GIB), true, Some(32)),
            (Some(sixteen), true, Some(16)),
            (Some(16 * GIB), true, Some(16)),
            (Some(8 * GIB), false, Some(8)),
            (Some(12 * GIB), false, Some(12)),
            (None, false, None),
        ] {
            assert_eq!(
                playback_with_generation_default(physical),
                on,
                "{physical:?}"
            );
            assert_eq!(gigabytes(physical), gb, "{physical:?}");
        }
    }
}
