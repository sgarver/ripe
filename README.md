# ripe

Secure, fast `vipe` clone in Rust: edit piped text in your `$EDITOR`, emit to stdout.

```bash
echo -e "one\ntwo" | ripe | cat
git log --oneline -20 | ripe | awk '{print $1}'
find . -name "*.tmp" | ripe --suffix .txt | xargs rm
```

## Install

```bash
cargo install --path .
# or
cargo build --release && install -Dm755 target/release/ripe ~/.local/bin/ripe
```

## Usage

```
ripe [--editor CMD] [--suffix SUFFIX]
```

- Editor: `--editor` > `$VISUAL` > `$EDITOR` > `vi`
- Exit `0` (`:wq`): edited content → stdout
- Non-zero (`:cq` in vim): abort, nothing emitted, exit code propagated
- `| head` safe: `BrokenPipe` treated as success
- Empty stdin (TTY): opens empty buffer

`--suffix .rs` gives the editor a hint for syntax highlighting.

## Security

- Single `0600` tempfile (`ripe-XXXXXX`), `O_EXCL` creation, chmod enforced.
- No shell: editor spawned directly, no `sh -c` expansion.
- Tempfile deleted *before* stdout write; nothing persists in `$TMPDIR`.
- Note: root can always read tempfiles; `0600` is max isolation on Linux.

## Why not `vipe`?

`vipe` (moreutils/Perl) is battle-tested and fine. `ripe` is one static
binary, faster startup, identical pipe semantics, smaller attack surface.
Use either.
