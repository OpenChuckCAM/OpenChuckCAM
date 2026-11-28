// SPDX-License-Identifier: AGPL-3.0-or-later
//! The file for a bug report (NFR-20; decision PROJUX-B-4): what it holds by
//! default and by choice, the home folder read as "~", the list shown before
//! saving, and a zip any tool reads. Generated folders and texts only.

// Test code may unwrap, index and panic (NFR-07 applies to library code).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use openchuck_app::bug_report::{self, Choice, Sources};

/// A scratch folder for one test, standing in for the user's home.
fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "openchuck-bug-report-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A home folder with two log files, a project and its model, and one
/// program file.
fn sources(name: &str) -> Sources {
    let home = home(name);
    let logs = home.join("logs");
    let log = logs.join("openchuckcam.log");
    let old = logs.join("openchuckcam.1.log");
    write(
        &log,
        &format!(
            "0.001 INFO start\n0.002 INFO opened {}\n",
            home.join("Projects").join("demo").display()
        ),
    );
    write(&old, "0.001 INFO an older run\n");
    let project = home.join("Projects").join("demo");
    write(
        &project.join("project.toml"),
        "schema = 30\nname = \"demo\"\n",
    );
    write(
        &project.join("setups").join("a.toml"),
        "name = \"Setup 1\"\n",
    );
    write(
        &project.join("models").join("part.stl"),
        "solid part\nendsolid\n",
    );
    write(
        &project.join(".openchuck-recovery").join("periodic-1"),
        "recovery",
    );
    let program = home.join("Out").join("demo.nc");
    write(&program, "G21\nM30\n");
    Sources {
        build: "OpenChuckCAM 0.70.0\ncommit abc1234\n".to_owned(),
        logs: vec![log, old],
        project: Some(project.clone()),
        models: vec![project.join("models").join("part.stl")],
        programs: vec![program],
        home: Some(home),
    }
}

/// The names of the entries a zip holds, read back with this test's own
/// reader, which also checks every entry's CRC and size.
fn read_zip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
    let u32_at =
        |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let end = bytes.len() - 22;
    assert_eq!(u32_at(end), 0x0605_4b50, "end of central directory");
    let count = u16_at(end + 10);
    let mut at = u32_at(end + 16) as usize;
    let mut out = Vec::new();
    for _ in 0..count {
        assert_eq!(u32_at(at), 0x0201_4b50, "central directory entry");
        let method = u16_at(at + 10);
        assert_eq!(method, 0, "stored, not compressed");
        let crc = u32_at(at + 16);
        let size = u32_at(at + 20) as usize;
        let name_len = u16_at(at + 28);
        let extra = u16_at(at + 30);
        let comment = u16_at(at + 32);
        let local = u32_at(at + 42) as usize;
        let name = String::from_utf8(bytes[at + 46..at + 46 + name_len].to_vec()).unwrap();
        assert_eq!(u32_at(local), 0x0403_4b50, "local header of {name}");
        let data_at = local + 30 + u16_at(local + 26) + u16_at(local + 28);
        let data = bytes[data_at..data_at + size].to_vec();
        assert_eq!(bug_report::crc32(&data), crc, "CRC of {name}");
        out.push((name, data));
        at += 46 + name_len + extra + comment;
    }
    out
}

fn names(entries: &[(String, Vec<u8>)]) -> Vec<&str> {
    entries.iter().map(|(n, _)| n.as_str()).collect()
}

#[test]
fn req_nfr_20_by_default_the_file_holds_the_build_and_the_logs_and_no_project() {
    let sources = sources("default");
    let entries = bug_report::entries(&sources, Choice::default());
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "build-information.txt",
            "logs/openchuckcam.log",
            "logs/openchuckcam.1.log",
            "contents.txt",
        ]
    );
}

#[test]
fn req_nfr_20_the_project_models_and_programs_are_in_only_when_chosen() {
    let sources = sources("chosen");
    let project = bug_report::entries(
        &sources,
        Choice {
            project: true,
            ..Choice::default()
        },
    );
    let names: Vec<&str> = project.iter().map(|e| e.name.as_str()).collect();
    // The project's own files: no model, no recovery data.
    assert!(names.contains(&"project/project.toml"), "{names:?}");
    assert!(names.contains(&"project/setups/a.toml"), "{names:?}");
    assert!(!names.iter().any(|n| n.contains("part.stl")), "{names:?}");
    assert!(!names.iter().any(|n| n.contains("recovery")), "{names:?}");
    assert!(
        !names.iter().any(|n| n.starts_with("programs/")),
        "{names:?}"
    );

    let all = bug_report::entries(
        &sources,
        Choice {
            project: true,
            models: true,
            programs: true,
        },
    );
    let names: Vec<&str> = all.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"models/part.stl"), "{names:?}");
    assert!(names.contains(&"programs/demo.nc"), "{names:?}");
}

#[test]
fn req_nfr_20_req_nfr_09_the_home_folder_reads_as_a_tilde_in_names_and_text() {
    let sources = sources("home");
    let home = sources.home.clone().unwrap();
    let entries = bug_report::entries(
        &sources,
        Choice {
            project: true,
            models: true,
            programs: true,
        },
    );
    let home_text = home.display().to_string();
    for entry in &entries {
        assert!(!entry.name.contains(&home_text), "{}", entry.name);
        let text = String::from_utf8_lossy(&entry.bytes);
        assert!(!text.contains(&home_text), "{}: {text}", entry.name);
    }
    let log = entries
        .iter()
        .find(|e| e.name == "logs/openchuckcam.log")
        .unwrap();
    let text = String::from_utf8_lossy(&log.bytes);
    let sep = std::path::MAIN_SEPARATOR;
    assert!(
        text.contains(&format!("opened ~{sep}Projects{sep}demo")),
        "{text}"
    );
    // The same with Windows separators in the text, whatever the platform.
    assert_eq!(
        bug_report::without_home(
            r"opened C:\Users\pat\Projects\x",
            Some(Path::new(r"C:\Users\pat"))
        ),
        r"opened ~\Projects\x"
    );
}

#[test]
fn req_nfr_20_the_list_shown_before_saving_names_exactly_what_is_written() {
    let sources = sources("list");
    let choice = Choice {
        project: true,
        ..Choice::default()
    };
    let entries = bug_report::entries(&sources, choice);
    let shown = bug_report::preview(&entries);
    let dir = home("list-out");
    let out = dir.join("report.zip");
    bug_report::write_zip(&out, &entries, bug_report::FIXED_TIME).unwrap();
    let read = read_zip(&std::fs::read(&out).unwrap());
    let shown_names: Vec<&str> = shown.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(shown_names, names(&read));
    // Every size shown is the size written.
    for ((_, size), (_, data)) in shown.iter().zip(&read) {
        assert_eq!(*size, data.len() as u64);
    }
    // The contents list in the file says what it holds, in plain words.
    let contents = read
        .iter()
        .find(|(n, _)| n == "contents.txt")
        .map(|(_, d)| String::from_utf8(d.clone()).unwrap())
        .unwrap();
    assert!(contents.contains("project/project.toml"), "{contents}");
    assert!(contents.contains("not included: model files"), "{contents}");
}

#[test]
fn req_nfr_20_the_zip_reads_back_and_is_the_same_bytes_for_the_same_inputs() {
    let sources = sources("zip");
    let choice = Choice {
        project: true,
        models: true,
        programs: true,
    };
    let dir = home("zip-out");
    let first = dir.join("a.zip");
    let second = dir.join("b.zip");
    let entries = bug_report::entries(&sources, choice);
    bug_report::write_zip(&first, &entries, bug_report::FIXED_TIME).unwrap();
    bug_report::write_zip(
        &second,
        &bug_report::entries(&sources, choice),
        bug_report::FIXED_TIME,
    )
    .unwrap();
    let a = std::fs::read(&first).unwrap();
    assert_eq!(a, std::fs::read(&second).unwrap());
    let read = read_zip(&a);
    assert_eq!(read.len(), entries.len());
    for ((name, data), entry) in read.iter().zip(&entries) {
        assert_eq!(name, &entry.name);
        assert_eq!(data, &entry.bytes);
    }
    // Where `unzip` is installed, it tests the file too.
    if let Ok(out) = std::process::Command::new("unzip")
        .arg("-t")
        .arg(&first)
        .output()
    {
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

#[test]
fn req_nfr_20_req_nfr_07_a_missing_log_is_said_not_a_crash() {
    let mut sources = sources("missing");
    sources
        .logs
        .push(PathBuf::from("/nowhere/openchuckcam.2.log"));
    sources.project = Some(PathBuf::from("/nowhere/project"));
    let entries = bug_report::entries(
        &sources,
        Choice {
            project: true,
            ..Choice::default()
        },
    );
    let contents = entries
        .iter()
        .find(|e| e.name == "contents.txt")
        .map(|e| String::from_utf8(e.bytes.clone()).unwrap())
        .unwrap();
    assert!(
        contents.contains("could not be read: openchuckcam.2.log"),
        "{contents}"
    );
    assert!(
        contents.contains("could not be read: project"),
        "{contents}"
    );
    // A folder that cannot be written is an error, not a panic.
    let err = bug_report::write_zip(
        Path::new("/nowhere/at/all/report.zip"),
        &entries,
        bug_report::FIXED_TIME,
    )
    .unwrap_err();
    assert!(err.contains("report.zip"), "{err}");
}

#[test]
fn req_nfr_20_crc32_matches_the_standard_check_value() {
    // The CRC-32 check value of "123456789" (IEEE 802.3).
    assert_eq!(bug_report::crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(bug_report::crc32(b""), 0);
}

#[test]
fn req_nfr_20_no_log_found_is_said_in_the_file() {
    let mut sources = sources("no-logs");
    sources.logs.clear();
    let entries = bug_report::entries(&sources, Choice::default());
    let contents = String::from_utf8(entries.last().unwrap().bytes.clone()).unwrap();
    assert!(
        contents.contains("not included: log files: none were found"),
        "{contents}"
    );
}
