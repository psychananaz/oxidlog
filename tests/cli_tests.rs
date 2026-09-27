use assert_cmd::Command;
use serde_json::Value;
use std::{fs, time::Duration};
use tempfile::{tempdir, TempDir};

// Each child process gets its own journal directory; never change the test
// runner's environment or use the developer's real journal.
fn command(dir: &TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xlog"));
    cmd.env("XLOG_HOME", dir.path())
        .env("NO_COLOR", "1")
        .current_dir(dir.path())
        .timeout(Duration::from_secs(10));
    cmd
}

fn journal() -> TempDir {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("journal.json"), "[]").unwrap();
    fs::write(
        dir.path().join("config.toml"),
        "[journal_cfg]\ninline_tags = false\nshow_time = false\nexport_dir = 'exports'\n",
    )
    .unwrap();
    dir
}

fn add_entry(dir: &TempDir) {
    command(dir)
        .args(["add", "Test entry #test"])
        .assert()
        .success();
}

fn entries(dir: &TempDir) -> Value {
    serde_json::from_str(&fs::read_to_string(dir.path().join("journal.json")).unwrap()).unwrap()
}

fn stdout(dir: &TempDir, args: &[&str]) -> String {
    let assertion = command(dir).args(args).assert().success();
    String::from_utf8(assertion.get_output().stdout.clone()).unwrap()
}

#[test]
fn test_xlog_init_requires_terminal() {
    // Init uses dialoguer's terminal UI; piped input cannot answer its prompts.
    let dir = tempdir().unwrap();
    let assertion = command(&dir)
        .args(["init", "--export-dir", "exports"])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr);
    assert!(stderr.contains("Failed to get timestamp preference"));
    assert!(!dir.path().join("journal.json").exists());
}

#[test]
fn test_xlog_init_protects_existing_journal() {
    let dir = journal();
    add_entry(&dir);
    let original = entries(&dir);
    let assertion = command(&dir)
        .args(["init", "--export-dir", "exports"])
        .assert()
        .failure();
    assert!(String::from_utf8_lossy(&assertion.get_output().stderr)
        .contains("Failed to get user confirmation"));
    assert_eq!(entries(&dir), original);
}

#[test]
fn test_xlog_missing_journal() {
    let dir = tempdir().unwrap();
    let assertion = command(&dir).arg("view").assert().failure();
    assert!(String::from_utf8_lossy(&assertion.get_output().stderr).contains("Journal not found"));
}

#[test]
fn test_xlog_add() {
    let dir = journal();
    add_entry(&dir);
    let saved = entries(&dir);
    assert_eq!(saved.as_array().unwrap().len(), 1);
    assert_eq!(saved[0]["id"], 0);
    assert_eq!(saved[0]["body"], "Test entry");
    assert_eq!(saved[0]["tags"][0]["name"], "test");
}

#[test]
fn test_xlog_view() {
    let dir = journal();
    add_entry(&dir);
    assert!(stdout(&dir, &["view"]).contains("Test entry"));
    let detail = stdout(&dir, &["view", "0"]);
    assert!(detail.contains("Entry #0"));
    assert!(detail.contains("Test entry"));
    assert!(detail.contains("Tags: #test"));
}

#[test]
fn test_xlog_remove() {
    let dir = journal();
    add_entry(&dir);
    assert!(stdout(&dir, &["remove", "0"]).contains("Entry 0 removed"));
    assert!(entries(&dir).as_array().unwrap().is_empty());
}

#[test]
fn test_xlog_edit() {
    let dir = journal();
    add_entry(&dir);
    command(&dir)
        .args(["edit", "0"])
        .write_stdin("  Updated entry  \nupdated\n")
        .assert()
        .success();
    let saved = entries(&dir);
    assert_eq!(saved[0]["body"], "Updated entry");
    assert_eq!(saved[0]["tags"][0]["name"], "updated");
}

#[test]
fn test_xlog_search() {
    let dir = journal();
    add_entry(&dir);
    assert!(stdout(&dir, &["search", "Test"]).contains("Test entry"));
    assert!(!stdout(&dir, &["search", "absent"]).contains("Test entry"));
}

#[test]
fn test_xlog_export() {
    let dir = journal();
    add_entry(&dir);
    assert!(stdout(&dir, &["export", "json"]).contains("Journal exported successfully"));
    let paths: Vec<_> = fs::read_dir(dir.path().join("exports"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(paths.len(), 1);
    let exported: Value = serde_json::from_str(&fs::read_to_string(&paths[0]).unwrap()).unwrap();
    assert_eq!(exported, entries(&dir));
}

#[test]
fn test_xlog_backup() {
    let dir = journal();
    add_entry(&dir);
    let original = entries(&dir);
    assert!(stdout(&dir, &["backup", "create"]).contains("Backup created"));
    let backup: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("journal.json.bak")).unwrap())
            .unwrap();
    assert_eq!(backup, original);
    fs::write(dir.path().join("journal.json"), "[]").unwrap();
    assert!(stdout(&dir, &["backup", "restore"]).contains("Backup restored"));
    assert_eq!(entries(&dir), original);
}

#[test]
fn ids_remain_unique_after_deletion_and_reload() {
    let dir = journal();
    assert!(stdout(&dir, &["add", "first"]).contains("Entry #0 added"));
    add_entry(&dir);
    add_entry(&dir);
    stdout(&dir, &["remove", "1"]);
    assert!(stdout(&dir, &["add", "fourth"]).contains("Entry #3 added"));
    let saved = entries(&dir);
    let ids: Vec<_> = saved
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, [0, 2, 3]);
}

#[test]
fn duplicate_ids_are_rejected_without_modifying_journal() {
    let dir = journal();
    add_entry(&dir);
    let entry = entries(&dir)[0].clone();
    let invalid = serde_json::json!([entry, entry]);
    fs::write(dir.path().join("journal.json"), invalid.to_string()).unwrap();
    let assertion = command(&dir).args(["add", "new"]).assert().failure();
    assert!(String::from_utf8_lossy(&assertion.get_output().stderr).contains("Duplicate entry ID"));
    assert_eq!(entries(&dir), invalid);
}

#[test]
fn bulk_removal_saves_once_and_accepts_gaps() {
    let dir = journal();
    for _ in 0..4 {
        add_entry(&dir);
    }
    stdout(&dir, &["remove", "1"]);
    let original = entries(&dir);
    stdout(&dir, &["remove", "--range", "0..2"]);
    assert_eq!(entries(&dir).as_array().unwrap().len(), 1);
    assert_eq!(entries(&dir)[0]["id"], 3);
    let backup: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("journal.json.bak")).unwrap())
            .unwrap();
    assert_eq!(backup, original);
}

#[test]
fn invalid_removal_does_not_partially_delete() {
    let dir = journal();
    add_entry(&dir);
    let original = entries(&dir);
    for args in [
        vec!["remove", "9", "--range", "0..9"],
        vec!["remove", "--range", "2..0"],
        vec!["remove", "0", "--from", "bad-date"],
    ] {
        command(&dir).args(args).assert().failure();
        assert_eq!(entries(&dir), original);
    }
}

#[test]
fn invalid_dates_fail_even_on_empty_journals() {
    let dir = journal();
    for cmd in ["view", "search", "remove"] {
        for dates in [
            ["--from", "invalid", "--to", "2025-01-01"],
            ["--from", "2025-02-01", "--to", "2025-01-01"],
        ] {
            let mut process = command(&dir);
            process.arg(cmd);
            if cmd == "search" {
                process.arg("anything");
            }
            let assertion = process.args(dates).assert().failure();
            let error = String::from_utf8_lossy(&assertion.get_output().stderr);
            assert!(!error.contains("panicked"), "{error}");
        }
    }
}

#[test]
fn help_and_init_are_accessible_with_invalid_config() {
    let dir = journal();
    fs::write(dir.path().join("config.toml"), "broken = [").unwrap();
    command(&dir).arg("--help").assert().success();
    let assertion = command(&dir)
        .args(["init", "--export-dir", "exports"])
        .assert()
        .failure();
    assert!(String::from_utf8_lossy(&assertion.get_output().stderr)
        .contains("Failed to get user confirmation"));
}

#[test]
fn loading_commented_config_does_not_create_export_directory() {
    let dir = journal();
    let export = dir.path().join("not-created");
    let config = format!(
        "# valid TOML comment\n[journal_cfg]\nexport_dir = '{}'\n",
        export.display()
    );
    fs::write(dir.path().join("config.toml"), config).unwrap();
    command(&dir).arg("view").assert().success();
    assert!(!export.exists());
}

#[test]
fn backup_restores_a_corrupted_journal_and_rejects_corrupted_backup() {
    let dir = journal();
    add_entry(&dir);
    let original = entries(&dir);
    stdout(&dir, &["backup", "create"]);
    fs::write(dir.path().join("journal.json"), "broken").unwrap();
    stdout(&dir, &["backup", "restore"]);
    assert_eq!(entries(&dir), original);
    fs::write(dir.path().join("journal.json.bak"), "broken backup").unwrap();
    command(&dir).args(["backup", "restore"]).assert().failure();
    assert_eq!(entries(&dir), original);
}

#[test]
fn search_handles_repetition_unicode_and_case_without_ansi_in_pipes() {
    let dir = journal();
    stdout(&dir, &["add", "İ café CAFÉ cat cat"]);
    for query in ["café", "cat", "i"] {
        let output = stdout(&dir, &["search", query]);
        assert!(output.contains("İ café CAFÉ cat cat"));
        assert!(!output.contains('\u{1b}'));
    }
    assert!(stdout(&dir, &["search", "CAFÉ", "--case-sensitive"]).contains("1 entries found"));
    assert!(stdout(&dir, &["search", "Café", "--case-sensitive"]).contains("No entries found"));
    assert!(stdout(&dir, &["search", "z", "--fuzzy"]).contains("No entries found"));
}

#[test]
fn edit_preserves_defaults_and_eof_does_not_save() {
    let dir = journal();
    add_entry(&dir);
    let original = entries(&dir);
    command(&dir)
        .args(["edit", "0"])
        .write_stdin("\n\n")
        .assert()
        .success();
    assert_eq!(entries(&dir), original);
    command(&dir)
        .args(["edit", "0"])
        .write_stdin("unsaved\n")
        .assert()
        .failure();
    assert_eq!(entries(&dir), original);
    command(&dir)
        .args(["edit", "0"])
        .write_stdin("\n-\n")
        .assert()
        .success();
    assert!(entries(&dir)[0]["tags"].as_array().unwrap().is_empty());
}

#[test]
fn add_and_edit_share_inline_tag_rules() {
    let dir = journal();
    fs::write(
        dir.path().join("config.toml"),
        "[journal_cfg]\ninline_tags = true\n",
    )
    .unwrap();
    stdout(&dir, &["add", "body #work #work"]);
    assert_eq!(entries(&dir)[0]["body"], "body");
    command(&dir)
        .args(["edit", "0"])
        .write_stdin("new body #home #home\n#work work\n")
        .assert()
        .success();
    let saved = entries(&dir);
    assert_eq!(saved[0]["body"], "new body");
    assert_eq!(
        saved[0]["tags"],
        serde_json::json!([{"name":"work"}, {"name":"home"}])
    );
}

#[test]
fn recent_view_applies_filters_and_missing_id_is_an_error() {
    let dir = journal();
    stdout(&dir, &["add", "first #work"]);
    stdout(&dir, &["add", "second #home"]);
    let output = stdout(&dir, &["view", "--recent", "--tags", "work"]);
    assert!(output.contains("first"));
    assert!(!output.contains("second"));
    command(&dir).args(["view", "999"]).assert().failure();
    command(&dir)
        .args(["view", "0", "--recent"])
        .assert()
        .failure();
}

#[test]
fn errors_include_command_and_preserve_underlying_failure_details() {
    let dir = journal();
    let assertion = command(&dir).args(["edit", "123"]).assert().failure();
    assert!(String::from_utf8_lossy(&assertion.get_output().stderr)
        .contains("edit: Entry with ID 123 not found"));
    fs::write(dir.path().join("journal.json"), "invalid json").unwrap();
    let assertion = command(&dir)
        .args(["search", "anything"])
        .assert()
        .failure();
    let error = String::from_utf8_lossy(&assertion.get_output().stderr);
    assert!(error.contains("search: Invalid journal"));
    assert!(error.contains("journal.json"));
    assert!(!error.contains("panicked"));
}

#[test]
fn colored_search_highlights_only_original_body_matches() {
    let dir = journal();
    stdout(&dir, &["add", "İ İ #i"]);
    let assertion = command(&dir)
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .args(["search", "i"])
        .assert()
        .success();
    let output = String::from_utf8_lossy(&assertion.get_output().stdout);
    assert_eq!(output.matches("\u{1b}[42mİ\u{1b}[0m").count(), 2);
    // Tags are metadata and must not receive body-search highlighting.
    assert_eq!(output.matches("\u{1b}[42m").count(), 2);
}

#[test]
fn tag_display_is_independent_of_storage_and_can_be_switched() {
    for initial in [false, true] {
        let dir = journal();
        fs::write(
            dir.path().join("config.toml"),
            format!("[journal_cfg]\ninline_tags = {initial}\n"),
        )
        .unwrap();
        stdout(&dir, &["add", "hello #work"]);
        command(&dir)
            .args(["edit", "0"])
            .write_stdin("updated #home\n-\n")
            .assert()
            .success();
        let original = fs::read(dir.path().join("journal.json")).unwrap();
        assert_eq!(entries(&dir)[0]["body"], "updated");
        assert_eq!(
            entries(&dir)[0]["tags"],
            serde_json::json!([{"name":"home"}])
        );
        for inline in [true, false] {
            fs::write(
                dir.path().join("config.toml"),
                format!("[journal_cfg]\ninline_tags = {inline}\n"),
            )
            .unwrap();
            for args in [vec!["view"], vec!["view", "0"], vec!["search", "updated"]] {
                let output = stdout(&dir, &args);
                if inline {
                    assert!(output.contains("updated #home"), "{output}");
                    assert!(!output.contains("Tags:"));
                } else {
                    assert!(!output.contains("updated #home"), "{output}");
                    assert!(output.contains("home"));
                }
            }
            assert_eq!(fs::read(dir.path().join("journal.json")).unwrap(), original);
        }
    }
}
