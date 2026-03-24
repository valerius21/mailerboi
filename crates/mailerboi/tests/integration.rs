use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn test_config() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-config.toml")
}

fn mailerboi() -> Command {
    let mut cmd = Command::cargo_bin("mailerboi").unwrap();
    cmd.arg("--config").arg(test_config()).arg("--insecure");
    cmd
}

#[test]
fn list_accounts_table_output() {
    let mut cmd = mailerboi();
    cmd.arg("list-accounts");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("test@localhost"));
}

#[test]
fn list_accounts_json_output() {
    let mut cmd = mailerboi();
    cmd.arg("--output").arg("json").arg("list-accounts");
    let output = cmd.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_array());
}

#[test]
fn list_accounts_toon_output() {
    let mut cmd = mailerboi();
    cmd.arg("--output").arg("toon").arg("list-accounts");
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
#[ignore]
fn doctor_all_checks_pass() {
    let mut cmd = mailerboi();
    cmd.arg("doctor");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DNS"))
        .stdout(predicate::str::contains("Auth"));
}

#[test]
#[ignore]
fn check_shows_inbox_status() {
    let mut cmd = mailerboi();
    cmd.arg("check");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("INBOX"))
        .stdout(predicate::str::contains("test"));
}

#[test]
#[ignore]
fn folders_shows_inbox() {
    let mut cmd = mailerboi();
    cmd.arg("folders");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("INBOX"));
}

#[test]
#[ignore]
fn folders_json_output() {
    let mut cmd = mailerboi();
    cmd.arg("--output").arg("json").arg("folders");
    let output = cmd.assert().success().get_output().stdout.clone();
    let text = String::from_utf8(output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_array());
    assert!(!parsed.as_array().unwrap().is_empty());
}

#[test]
#[ignore]
fn list_empty_inbox() {
    let mut cmd = mailerboi();
    cmd.arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No messages").or(predicate::str::contains("UID")));
}

#[test]
#[ignore]
fn search_unseen_returns_results_or_empty() {
    let mut cmd = mailerboi();
    cmd.arg("search").arg("--unseen");
    cmd.assert().success();
}

#[test]
#[ignore]
fn draft_creates_in_drafts_folder() {
    let mut cmd = mailerboi();
    cmd.arg("draft")
        .arg("--subject")
        .arg("Integration Test Draft")
        .arg("--mailbox")
        .arg("INBOX")
        .arg("--body")
        .arg("This is a test draft");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Draft created"));
}

#[test]
fn missing_config_file_exits_with_error() {
    let mut cmd = Command::cargo_bin("mailerboi").unwrap();
    cmd.arg("--config")
        .arg("/nonexistent/config.toml")
        .arg("list-accounts");
    cmd.assert().failure();
}

#[test]
fn unknown_subcommand_exits_with_error() {
    let mut cmd = Command::cargo_bin("mailerboi").unwrap();
    cmd.arg("nonexistent-command");
    cmd.assert().failure();
}

#[test]
fn help_shows_all_subcommands() {
    let mut cmd = Command::cargo_bin("mailerboi").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("list-accounts"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("folders"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("move"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("flag"))
        .stdout(predicate::str::contains("download"))
        .stdout(predicate::str::contains("draft"));
}
