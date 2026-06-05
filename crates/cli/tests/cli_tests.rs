use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

const PAYLOAD: &str = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n:61:2401012401D100,00NTRF//REF\r\n:86:test debit\r\n:62F:C240101EUR900,00\r\n";

fn tmp(name: &str) -> String {
    std::env::temp_dir().join(format!("x940_cli_{}", name)).to_string_lossy().to_string()
}

#[test]
fn transform_json_output() {
    let input = tmp("json.sta");
    std::fs::write(&input, PAYLOAD).unwrap();

    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", &input, "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("transactionReference"))
        .stdout(predicate::str::contains("ACCT"));

    let _ = std::fs::remove_file(&input);
}

#[test]
fn transform_csv_output() {
    let input = tmp("csv.sta");
    std::fs::write(&input, PAYLOAD).unwrap();

    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", &input, "--format", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("\u{FEFF}"))
        .stdout(predicate::str::contains("ACCT"))
        .stdout(predicate::str::contains("-100.00"));

    let _ = std::fs::remove_file(&input);
}

#[test]
fn transform_camt053_output() {
    let input = tmp("camt053.sta");
    std::fs::write(&input, PAYLOAD).unwrap();

    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", &input, "--format", "camt053"])
        .assert()
        .success()
        .stdout(predicate::str::contains("camt.053"))
        .stdout(predicate::str::contains("<CdtDbtInd>"));

    let _ = std::fs::remove_file(&input);
}

#[test]
fn transform_with_resolver() {
    let input = tmp("resolver.sta");
    std::fs::write(&input, PAYLOAD).unwrap();

    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", &input, "--format", "json", "--resolver", "gvc"])
        .assert()
        .success();

    let _ = std::fs::remove_file(&input);
}

#[test]
fn transform_pipe_input() {
    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", "--format", "json"])
        .write_stdin(PAYLOAD)
        .assert()
        .success()
        .stdout(predicate::str::contains("transactionReference"));
}

#[test]
fn transform_multi_format() {
    let input = tmp("multi.sta");
    let prefix = tmp("multi_out");
    std::fs::write(&input, PAYLOAD).unwrap();

    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", &input, "--format", "json,csv", "--output-prefix", &prefix])
        .assert()
        .success();

    assert!(PathBuf::from(format!("{}.json", prefix)).exists());
    assert!(PathBuf::from(format!("{}.csv", prefix)).exists());

    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(format!("{}.json", prefix));
    let _ = std::fs::remove_file(format!("{}.csv", prefix));
}

#[test]
fn transform_file_output() {
    let input = tmp("fileout.sta");
    let output = tmp("fileout.json");
    std::fs::write(&input, PAYLOAD).unwrap();

    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", &input, "--format", "json", "--output", &output])
        .assert()
        .success();

    let content = std::fs::read_to_string(&output).unwrap();
    assert!(content.contains("transactionReference"));

    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn transform_empty_input_errors() {
    Command::cargo_bin("x940")
        .unwrap()
        .args(["transform", "--format", "json"])
        .write_stdin("")
        .assert()
        .failure();
}

#[test]
fn transform_missing_format_errors() {
    let input = tmp("noformat.sta");
    std::fs::write(&input, PAYLOAD).unwrap();

    Command::cargo_bin("x940").unwrap().args(["transform", &input]).assert().failure();

    let _ = std::fs::remove_file(&input);
}
