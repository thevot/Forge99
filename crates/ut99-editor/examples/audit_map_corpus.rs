//! Read every UNR in a directory and report successes and errors individually.
//! Usage: cargo run -p ut99-editor --example audit_map_corpus -- <maps-dir> > report.json
use std::{env, fs, path::PathBuf};
use ut99_format::OfficialMapDna;

fn main() -> anyhow::Result<()> {
    let dir = env::args_os().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("UnrealTournament/Maps"));
    let mut paths = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("unr")) {
            paths.push(path);
        }
    }
    paths.sort();
    let mut records = Vec::with_capacity(paths.len());
    for path in paths {
        let filename = path.file_name().unwrap().to_string_lossy().into_owned();
        eprintln!("Reading {filename}");
        let record = match OfficialMapDna::from_unr_file(&path) {
            Ok(profile) => {
                let status = if profile.read_issues.is_empty() { "parsed" } else { "parsed_with_warnings" };
                serde_json::json!({ "file": filename, "status": status, "profile": profile })
            },
            Err(error) => serde_json::json!({ "file": filename, "status": "error", "error": format!("{error:#}") }),
        };
        records.push(record);
    }
    let failures = records.iter().filter(|r| r["status"] == "error").count();
    let partial = records.iter().filter(|r| r["status"] == "parsed_with_warnings").count();
    let report = serde_json::json!({
        "schema_version": 1, "directory": dir, "count": records.len(),
        "parse_failures": failures, "parsed_with_warnings": partial,
        "note": "An empty issue list is not proof of complete decoding; validate independently.",
        "maps": records,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
