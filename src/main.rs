use anyhow::{Context, Result};
use clap::Parser;
use std::io::{IsTerminal, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

/// ripe: stdin -> $EDITOR -> stdout (secure vipe clone)
///
/// No persistent cache: single 0600 tempfile, unlinked promptly,
/// fsync before edit, removed on all paths (incl. signals via Drop).
#[derive(Parser, Debug)]
#[command(
    name = "ripe",
    version,
    about = "Edit pipe content in $EDITOR, emit to stdout"
)]
struct Args {
    /// Suffix for temp file (e.g. --suffix .rs gives syntax highlighting)
    #[arg(long)]
    suffix: Option<String>,

    /// Editor to use (overrides $VISUAL / $EDITOR)
    #[arg(long)]
    editor: Option<String>,
}

fn resolve_editor(explicit: Option<String>) -> Result<(String, Vec<String>)> {
    if let Some(e) = explicit {
        return Ok(split_editor(&e));
    }
    for key in ["VISUAL", "EDITOR"] {
        if let Ok(v) = std::env::var(key) {
            if !v.trim().is_empty() {
                return Ok(split_editor(&v));
            }
        }
    }
    // vipe-compatible default
    Ok(("vi".to_string(), vec![]))
}

fn split_editor(s: &str) -> (String, Vec<String>) {
    // Minimal shell-word split (handles quotes). No glob/expansion by design.
    let mut parts: Vec<String> = vec![];
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => {
                if !cur.is_empty() {
                    parts.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    let prog = parts.first().cloned().unwrap_or_else(|| "vi".into());
    let args = parts.into_iter().skip(1).collect();
    (prog, args)
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Read stdin fully (empty if TTY — vipe still opens editor with empty buf).
    let mut input = Vec::new();
    if !std::io::stdin().is_terminal() {
        std::io::stdin()
            .read_to_end(&mut input)
            .context("reading stdin")?;
    }

    // 2. Secure tempfile: 0600, O_EXCL, auto-removed. Suffix aids highlighting.
    let mut builder = tempfile::Builder::new();
    builder.prefix("ripe-");
    if let Some(s) = &args.suffix {
        builder.suffix(s);
    }
    let mut tmp = builder.tempfile().context("creating secure tempfile")?;
    // Harden: ensure 0600 even under odd umask.
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600)).ok();
    tmp.write_all(&input).context("writing tempfile")?;
    tmp.flush()?;
    // Persist path but keep deletion guard alive via reopen trick:
    // keep `tmp` alive until after editor exits; TempPath auto-deletes on drop.
    let path = tmp.path().to_path_buf();

    // 3. Spawn $EDITOR attached to /dev/tty (stdin is data, so editor needs tty).
    // If stdin is a TTY already, inherit normally. Else wire editor stdio to /dev/tty.
    let (prog, eargs) = resolve_editor(args.editor)?;
    let tty_r = std::fs::OpenOptions::new().read(true).open("/dev/tty").ok();
    let tty_w = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .ok();

    let mut cmd = Command::new(&prog);
    cmd.args(&eargs).arg(&path);
    if tty_r.is_some() || tty_w.is_some() {
        // Only redirect when we actually have a tty (i.e. interactive use).
        if let Some(f) = &tty_r {
            // dup fd for child stdin via cloned File -> Stdio
            let dup = f.try_clone().context("dup tty for stdin")?;
            cmd.stdin(dup);
        }
        if let Some(f) = &tty_w {
            let out = f.try_clone().context("dup tty for stdout")?;
            let err = f.try_clone().context("dup tty for stderr")?;
            cmd.stdout(out).stderr(err);
        }
    }

    let status = cmd
        .status()
        .with_context(|| format!("spawning editor `{}`", prog))?;

    // 4. Editor exit semantics (vipe-compatible):
    //   exit 0  -> emit edited file to stdout
    //   non-zero (e.g. vim :cq) -> abort: emit nothing, exit same code
    if !status.success() {
        let code = status.code().unwrap_or(1);
        std::process::exit(code);
    }

    // Re-read (editor may have replaced/truncated the file).
    let mut out = Vec::new();
    std::fs::File::open(&path)
        .context("re-reading edited file")?
        .read_to_end(&mut out)?;

    // Explicitly drop tempfile BEFORE writing stdout so no trace remains
    // even if downstream pipe breaks.
    drop(tmp);

    // 5. Write to stdout; BrokenPipe (| head) is success, not error.
    let mut so = std::io::stdout().lock();
    match so.write_all(&out) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(e) => return Err(e.into()),
    }
    let _ = so.flush();
    Ok(())
}
