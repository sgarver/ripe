//! Integration tests using fake $EDITOR scripts (no TTY needed).
use std::io::Write;
use std::process::{Command, Stdio};

fn ripe_bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // deps -> debug
    p.pop();
    p.join("ripe")
}

fn write_fake_editor(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    let path = dir.join("fake-editor.sh");
    std::fs::write(&path, format!("#!/bin/sh\n{}", body)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn run_ripe(stdin: &str, editor: &str) -> (String, i32) {
    let mut child = Command::new(ripe_bin())
        .env("EDITOR", editor)
        .env("VISUAL", "")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn passthrough_when_editor_noop() {
    let tmp = tempfile::tempdir().unwrap();
    // `:` is a shell no-op that leaves the file untouched.
    let ed = write_fake_editor(tmp.path(), ": \"$1\"\n");
    let (out, code) = run_ripe("hello\nworld\n", ed.to_str().unwrap());
    assert_eq!(code, 0);
    assert_eq!(out, "hello\nworld\n");
}

#[test]
fn editor_can_modify_content() {
    let tmp = tempfile::tempdir().unwrap();
    let ed = write_fake_editor(tmp.path(), "printf 'edited\\n' > \"$1\"\n");
    let (out, code) = run_ripe("original\n", ed.to_str().unwrap());
    assert_eq!(code, 0);
    assert_eq!(out, "edited\n");
}

#[test]
fn nonzero_exit_aborts_with_no_output() {
    let tmp = tempfile::tempdir().unwrap();
    let ed = write_fake_editor(tmp.path(), "exit 3\n");
    let (out, code) = run_ripe("keep me\n", ed.to_str().unwrap());
    assert_eq!(code, 3);
    assert_eq!(out, "");
}

#[test]
fn empty_stdin_still_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let ed = write_fake_editor(tmp.path(), "printf 'new\\n' > \"$1\"\n");
    // Empty stdin: close immediately; editor writes fresh content.
    let (out, code) = run_ripe("", ed.to_str().unwrap());
    assert_eq!(code, 0);
    assert_eq!(out, "new\n");
}
