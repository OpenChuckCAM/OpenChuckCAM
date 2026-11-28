// SPDX-License-Identifier: AGPL-3.0-or-later
//! The file for a bug report (NFR-20; decision PROJUX-B-4): one zip, written
//! on the user's computer, that holds the build information (NFR-16) and the
//! diagnostic log with its action list (NFR-17, NFR-18) and, only when the
//! user ticks them, the project's own files, its model files and its program
//! files. The user's home folder reads as "~" in every entry name and text.
//! The list of what the file holds is shown before it is written, and is in
//! the file too, as `contents.txt`. Nothing is sent anywhere (NFR-09): this
//! module only reads and writes local files.
//!
//! The zip is written by this module, entries stored uncompressed, so no
//! compression library is needed (the owner's answer Q4b A, relayed). A
//! fixed entry time keeps the bytes the same for the same inputs.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// What goes into the file besides the build information and the logs,
/// which always do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Choice {
    /// The project's own files (its settings, setups, tools and operations),
    /// without its model files or its recovery data.
    pub project: bool,
    /// The project's model files.
    pub models: bool,
    /// The program files last written for it.
    pub programs: bool,
}

/// Where the file's contents come from.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// The build information, as the About window's Copy gives it.
    pub build: String,
    /// The log files, newest first.
    pub logs: Vec<PathBuf>,
    /// The open project's folder.
    pub project: Option<PathBuf>,
    /// The project's model files.
    pub models: Vec<PathBuf>,
    /// The program files last written.
    pub programs: Vec<PathBuf>,
    /// The user's home folder, read as "~" in the file.
    pub home: Option<PathBuf>,
}

/// One entry of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its name in the zip, with `/` between folders.
    pub name: String,
    /// Its bytes.
    pub bytes: Vec<u8>,
}

/// The time every entry carries: 1 January 2026, 00:00, so the same inputs
/// give the same bytes. A zip time has no time zone; the files' own dates
/// are not part of a report.
pub const FIXED_TIME: (u16, u16) = (dos_date(2026, 1, 1), 0);

const fn dos_date(year: u16, month: u16, day: u16) -> u16 {
    ((year - 1980) << 9) | (month << 5) | day
}

/// The folders of a project folder that are not the project's own files.
const NOT_PROJECT: [&str; 2] = ["models", ".openchuck-recovery"];

/// The entries of the file for `sources` and `choice`, in the order they
/// are written: the build information, the logs, then the chosen parts, and
/// last `contents.txt`, which lists them and says what is not included and
/// what could not be read (NFR-07).
pub fn entries(sources: &Sources, choice: Choice) -> Vec<Entry> {
    let home = sources.home.as_deref();
    let mut out = vec![Entry {
        name: "build-information.txt".to_owned(),
        bytes: without_home(&sources.build, home).into_bytes(),
    }];
    let mut unread = Vec::new();
    for log in &sources.logs {
        add_file(&mut out, &mut unread, log, "logs", home, true);
    }
    let mut left_out = Vec::new();
    if choice.project {
        match &sources.project {
            Some(folder) => add_project(&mut out, &mut unread, folder, home),
            None => left_out.push("the project: no project is open"),
        }
    } else {
        left_out.push("the project's files");
    }
    if choice.models {
        for model in &sources.models {
            add_file(&mut out, &mut unread, model, "models", home, false);
        }
    } else {
        left_out.push("model files");
    }
    if choice.programs {
        for program in &sources.programs {
            add_file(&mut out, &mut unread, program, "programs", home, true);
        }
    } else {
        left_out.push("program files");
    }
    let mut contents = String::from(
        "Files for an OpenChuckCAM bug report. Nothing was sent anywhere; \
         attach this file to your report yourself.\n\nIncluded:\n",
    );
    for entry in &out {
        let _ = writeln!(contents, "  {} ({} bytes)", entry.name, entry.bytes.len());
    }
    for what in &left_out {
        let _ = writeln!(contents, "not included: {what}");
    }
    for name in &unread {
        let _ = writeln!(contents, "could not be read: {name}");
    }
    let _ = writeln!(contents, "\nFolder names show your home folder as \"~\".");
    out.push(Entry {
        name: "contents.txt".to_owned(),
        bytes: contents.into_bytes(),
    });
    out
}

/// Adds one file under `folder`; a text file has the home folder replaced.
fn add_file(
    out: &mut Vec<Entry>,
    unread: &mut Vec<String>,
    path: &Path,
    folder: &str,
    home: Option<&Path>,
    text: bool,
) {
    let name = path
        .file_name()
        .map_or_else(|| "file".to_owned(), |n| n.to_string_lossy().into_owned());
    match std::fs::read(path) {
        Ok(bytes) => {
            let bytes = if text {
                match String::from_utf8(bytes) {
                    Ok(t) => without_home(&t, home).into_bytes(),
                    Err(e) => e.into_bytes(),
                }
            } else {
                bytes
            };
            out.push(Entry {
                name: format!("{folder}/{name}"),
                bytes,
            });
        }
        Err(_) => unread.push(name),
    }
}

/// Adds the project's own text files, in name order, without its model
/// files and recovery data.
fn add_project(out: &mut Vec<Entry>, unread: &mut Vec<String>, folder: &Path, home: Option<&Path>) {
    let mut files = Vec::new();
    if collect(folder, folder, &mut files).is_err() {
        unread.push("project".to_owned());
        return;
    }
    files.sort();
    for relative in files {
        let path = folder.join(&relative);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let bytes = match String::from_utf8(bytes) {
                    Ok(t) => without_home(&t, home).into_bytes(),
                    Err(e) => e.into_bytes(),
                };
                out.push(Entry {
                    name: format!("project/{relative}"),
                    bytes,
                });
            }
            Err(_) => unread.push(format!("project/{relative}")),
        }
    }
}

/// The files under `dir`, relative to `root` with `/` between folders,
/// leaving out the folders that are not the project's own files.
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() {
            if dir == root && NOT_PROJECT.contains(&name.as_str()) {
                continue;
            }
            collect(root, &path, out)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            let parts: Vec<String> = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(parts.join("/"));
        }
    }
    Ok(())
}

/// `text` with the home folder `home` written as "~", in either separator.
pub fn without_home(text: &str, home: Option<&Path>) -> String {
    let Some(home) = home else {
        return text.to_owned();
    };
    let shown = home.display().to_string();
    let shown = shown.trim_end_matches(['/', '\\']);
    if shown.len() < 2 {
        // A root folder as the home would hide every path; leave the text.
        return text.to_owned();
    }
    let mut result = text.replace(shown, "~");
    let other = if shown.contains('\\') {
        shown.replace('\\', "/")
    } else {
        shown.replace('/', "\\")
    };
    if other != shown {
        result = result.replace(&other, "~");
    }
    result
}

/// What the file will hold, as shown before it is written: each entry's
/// name and size.
pub fn preview(entries: &[Entry]) -> Vec<(String, u64)> {
    entries
        .iter()
        .map(|e| (e.name.clone(), e.bytes.len() as u64))
        .collect()
}

/// The CRC-32 of `bytes` (IEEE 802.3), as a zip needs it.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// The zip's bytes: each entry stored, then the central directory.
///
/// # Errors
/// When the file is too large for a plain zip (an entry or the whole over
/// 4 GiB, or more than 65,535 entries).
pub fn zip_bytes(entries: &[Entry], (date, time): (u16, u16)) -> Result<Vec<u8>, String> {
    let too_big = || "The file would be too large for a zip.".to_owned();
    let count = u16::try_from(entries.len()).map_err(|_| too_big())?;
    let mut out = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let offset = u32::try_from(out.len()).map_err(|_| too_big())?;
        let size = u32::try_from(entry.bytes.len()).map_err(|_| too_big())?;
        let name = entry.name.as_bytes();
        let name_len = u16::try_from(name.len()).map_err(|_| too_big())?;
        let crc = crc32(&entry.bytes);
        // Local file header: version 2.0, UTF-8 names (bit 11), stored.
        let common = |b: &mut Vec<u8>| {
            b.extend_from_slice(&20u16.to_le_bytes());
            b.extend_from_slice(&0x0800u16.to_le_bytes());
            b.extend_from_slice(&0u16.to_le_bytes());
            b.extend_from_slice(&time.to_le_bytes());
            b.extend_from_slice(&date.to_le_bytes());
            b.extend_from_slice(&crc.to_le_bytes());
            b.extend_from_slice(&size.to_le_bytes());
            b.extend_from_slice(&size.to_le_bytes());
            b.extend_from_slice(&name_len.to_le_bytes());
            b.extend_from_slice(&0u16.to_le_bytes());
        };
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        common(&mut out);
        out.extend_from_slice(name);
        out.extend_from_slice(&entry.bytes);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        common(&mut central);
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        central.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }
    let central_at = u32::try_from(out.len()).map_err(|_| too_big())?;
    let central_len = u32::try_from(central.len()).map_err(|_| too_big())?;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&central_len.to_le_bytes());
    out.extend_from_slice(&central_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    if u32::try_from(out.len()).is_err() {
        return Err(too_big());
    }
    Ok(out)
}

/// Writes the zip of `entries` to `path`, through a temporary file beside it
/// so a failed write never leaves half a file under the chosen name.
///
/// # Errors
/// A plain sentence naming the file and the reason (NFR-07).
pub fn write_zip(path: &Path, entries: &[Entry], time: (u16, u16)) -> Result<(), String> {
    let bytes = zip_bytes(entries, time)?;
    let shown = path.display();
    let fail = |e: std::io::Error| format!("Could not write {shown}: {e}.");
    let partial = path.with_extension("zip.part");
    let mut file = std::fs::File::create(&partial).map_err(fail)?;
    file.write_all(&bytes).map_err(fail)?;
    file.sync_all().map_err(fail)?;
    drop(file);
    std::fs::rename(&partial, path).map_err(|e| {
        let _ = std::fs::remove_file(&partial);
        fail(e)
    })
}
