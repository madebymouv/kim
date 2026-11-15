# ADR 0097: One patch version per development change; a small build directory; the root kim.exe

- Status: accepted
- Date: 2026-11-15
- Issue: maintainer request 2026-11-15 ("매번 개발 사항이 있을 때마다 0.0.1씩 오르도록",
  "용량을 정말 작게 유지", "항상 개발이 될때마다 root 경로의 kim.exe를 업데이트"); extends
  ADR 0089 (root kim.exe and the local mirror)

## Context

Every crate stayed at `0.0.1` through many deliveries, so an installed tool could not be told
apart from a newer build. The version was written by hand in several places: each crate's
`Cargo.toml`, the version requirement of every workspace path dependency (`^0.0.1` matches only
`0.0.1`), each tool's `capabilities.json` and `hotkeys.manifest.json` (Kim's test demands the
manifest equal the crate). Separately, `target/` had grown to about 111 GB: 51 GB of incremental
cache and 51 GB of debug test programs with dependency debug info.

## Decision

1. **Versions.** What ships under one version is a group: `kim` (kim, kim-manager, kim-setup),
   `subak` (subak, subak-term), `noeul` (noeul, noeul-settings), `ddalgak` (ddalgak,
   ddalgak-editor); each shared library (kim-design, kim-ui, kim-hotkeys, kim-winres) is its own.
   Every development change raises the changed groups by one patch step (`0.0.1` → `0.0.2`),
   plus every group built on a changed library. `cargo xtask bump --changed` does it from
   `git status` (changes under `tests/`, `examples/`, `benches/` and Markdown do not count; a
   group already raised since `HEAD` is not raised twice), writing the `Cargo.toml` versions,
   the path requirements, the two JSON files and `Cargo.lock`. `cargo xtask version-check`, part
   of `cargo xtask ci`, fails when they disagree. Minor and major versions stay a maintainer
   decision.
2. **A small `target/`.** The dev profile builds without the incremental cache and without debug
   info for dependencies (our own code keeps line tables, so panics still name file and line).
   Agents check the size of `target/` after each delivery and run `cargo clean` when it passes
   15 GB; only `target/` is ever cleaned.
3. **The root kim.exe.** After each delivery the release `kim.exe` and `kim-manager.exe` are
   copied to the repository root and `cargo xtask local-mirror` rebuilds `kim-local/` (ADR 0089),
   so a double-click installs the new versions.

## Consequences

- Kim Manager and `kim tools list` show which build is installed; an update is visible.
- Rebuilds after an edit compile each changed crate fully (no incremental reuse): somewhat
  slower, much smaller. A first build after `cargo clean` takes the time of a full CI build.
- The first application raised kim, subak, noeul and ddalgak to `0.0.2` for the design-system
  work of ADR 0092–0096.
