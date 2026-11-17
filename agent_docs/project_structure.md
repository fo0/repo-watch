# Project Structure

Moved from `CLAUDE.md` (context budget). Top-level directories are listed there in one line; this is the annotated tree.

```
src/        Svelte panel, lib/ = components, controllers, IPC map, demo; styles/ = ordered CSS
src-tauri/  Rust core crate + desktop bin, Tauri config, icons
tests/      Playwright specs (demo + IPC mock)
scripts/    icon generator, release/updater helpers + node:test
fixtures/   synthetic demo data
docs/       product docs, adr/, ARCHITECTURE.mmd
.github/    ci.yml (pull requests), desktop-release.yml (manual)
```

Module-level detail: `docs/ARCHITECTURE.md`, diagram `docs/ARCHITECTURE.mmd`.
