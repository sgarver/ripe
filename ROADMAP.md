# ripe — Roadmap

## Done
- [x] v0.1.0: stdin -> $EDITOR -> stdout, 0600 tempfile, no shell
- [x] Integration tests (4/4), CI (fmt/clippy/test/release)
- [x] GitHub release v0.1.0 with x86_64 binary
- [x] MIT license

## Next
1. **crates.io publish** — `cargo publish` (verify description/keywords/README rendering).
2. **Multi-arch releases** — aarch64 binary via cross / CI matrix; attach checksums.
3. **AUR package** — PKGBUILD for Arch/Omarchy (`ripe-bin` + `ripe` from source).
4. **Polish**
   - `--suffix` auto-detect from content (shebang/extension sniffing)
   - Man page (`ripe.1`) + shell completions (clap_complete)
   - `--check` / `--version` output tests
5. **Hardening**
   - Prefer `$TMPDIR` → `/run/user/$UID` → `/tmp` ordering
   - Document threat model (root can read all; hostile $EDITOR owns you)
6. **Promotion** — README demo GIF, submit to Omarchy TUIs list / Arch Wiki moreutils alternatives.
