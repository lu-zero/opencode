# Deno Migration — Phase 1 (Non-TUI CLI)

## What this does

Runs opencode's non-TUI commands (`run`, `serve`, `agent`, `models`, `db`, etc.) on Deno instead of Bun, using `bun-compat` as a Bun API polyfill. TUI commands print "not available" and exit.

No build step required — `deno run` resolves everything via the import map.

## Prerequisites

- [x] Deno 2.7+ with sloppy-imports support
- [x] `bun-compat` package cloned and available (symlinked into `packages/opencode/bun-compat`)
- [x] `node_modules/` present (from `bun install` — Deno uses `nodeModulesDir: "manual"`)

## How to run

```bash
deno run --sloppy-imports --no-check --allow-all \
  --config deno.json src/bootstrap.deno.ts <command>

# Examples:
deno run --sloppy-imports --no-check --allow-all --config deno.json src/bootstrap.deno.ts --help
deno run --sloppy-imports --no-check --allow-all --config deno.json src/bootstrap.deno.ts db path
deno run --sloppy-imports --no-check --allow-all --config deno.json src/bootstrap.deno.ts models
deno run --sloppy-imports --no-check --allow-all --config deno.json src/bootstrap.deno.ts run "explain this codebase"
```

## Files changed

### New files (untracked)

| File | Purpose |
|------|---------|
| `deno.json` | Import map: bun→bun-compat, bun:sqlite→shim, #db→db.node.ts, node: bare specifiers |
| `deno.lock` | Deno lock file |
| `bun-compat` | Symlink to bun-compat repo |
| `src/bootstrap.deno.ts` | Installs `globalThis.Bun` from bun-compat, polyfills `setInterval().unref()`, then imports index.deno.ts |
| `src/index.deno.ts` | Deno entry point — same as index.ts but imports TUI stubs |
| `src/cli/cmd/tui/attach.stub.ts` | TUI stub: prints "not available", exits |
| `src/cli/cmd/tui/thread.stub.ts` | TUI stub: prints "not available", exits |
| `src/util/stub.ts` | `bun:ffi` stubs (throw on use) |
| `shims/bun-sqlite.ts` | `bun:sqlite` → wraps `node:sqlite`'s `DatabaseSync` |
| `shims/bun-pty.ts` | `bun-pty` → throws |
| `shims/drizzle-bun-sqlite.ts` | `drizzle-orm/bun-sqlite` → re-exports from `drizzle-orm/node-sqlite` |
| `shims/drizzle-bun-sqlite-migrator.ts` | `drizzle-orm/bun-sqlite/migrator` → re-exports from `drizzle-orm/node-sqlite/migrator` |
| `src/**/*.txt.ts` (39 files) | `.txt` file wrappers — `export default "..."` so Deno can import them |

### Modified files (26 tracked)

| Change | Files |
|--------|-------|
| `.txt` → `.txt.ts` imports | 22 files: agent.ts, command/index.ts, session/system.ts, all tool/*.ts |
| `from "."` → `from "./index.ts"` | 5 files: session/{processor,revert,summary,compaction,prompt}.ts |

## Import map structure (deno.json)

```
"bun"           → ./bun-compat/src/from-bun.ts    (Bun API polyfill)
"bun:install"   → ./bun-compat/src/install.ts     (globalThis.Bun installer)
"bun:sqlite"    → ./shims/bun-sqlite.ts           (DatabaseSync wrapper)
"bun:ffi"       → ./src/util/stub.ts              (throws)
"bun-pty"       → ./shims/bun-pty.ts              (throws)
"#db"           → ./src/storage/db.node.ts        (node:sqlite + drizzle node-sqlite)
"drizzle-orm/bun-sqlite"         → ./shims/drizzle-bun-sqlite.ts
"drizzle-orm/bun-sqlite/migrator" → ./shims/drizzle-bun-sqlite-migrator.ts
```

Plus all `node:*` bare specifiers mapped, and `@parcel/watcher/wrapper`, `vscode-jsonrpc/node`.

## What works

- [x] `--help`, `--version`
- [x] `db path`
- [x] `models`, `models --help`
- [x] `run --help`, `serve --help`
- [x] All non-TUI command help text renders correctly

## Known limitations

1. **TUI unavailable** — `@opentui/core` uses `bun:ffi` (dlopen, ptr, JSCallback) to load a native Zig rendering library. Porting requires `Deno.dlopen` work.
2. **`.txt.ts` wrappers** — Deno can't import `.txt` files natively. The 39 `.txt.ts` wrappers embed the text as string exports. If upstream `.txt` files change, wrappers must be regenerated.
3. **`setInterval().unref()`** — Deno's `setInterval` doesn't return an object with `.unref()`. Bootstrapped with a polyfill. Only 2 non-TUI call sites affected.
4. **`--no-check` required** — Type errors from `drizzle-orm/bun-sqlite` vs `node-sqlite` type mismatch. Could be fixed with proper type shims.
5. **`bun-pty` unavailable** — PTY features (terminal sessions) won't work. The shim throws at runtime.

## Phase 2 checklist (TUI support)

- [ ] Port `@opentui/core`'s `bun:ffi` usage to `Deno.dlopen`
  - [ ] `dlopen` → `Deno.dlopen`
  - [ ] `ptr()` → `Deno.UnsafePointer.of()`
  - [ ] `toArrayBuffer()` → manual `ArrayBuffer` + `Deno.UnsafePointerView`
  - [ ] `JSCallback` → `Deno.UnsafeCallback`
- [ ] Build/port the native Zig rendering library for Deno FFI
- [ ] Test solid-js rendering pipeline
- [ ] Remove TUI stubs, restore real TUI command imports

## bun-compat repo checklist

- [x] All 29 tests passing
- [x] `from-bun.ts` exports: env, argv, $, Glob, spawn, spawnSync, file, write, sleep, serve, stdin, hash, stringWidth, wrapAnsi, password, TOML, YAML, JSONC, JSON5, JSONL, fileURLToPath, pathToFileURL, which, Database, SystemError
- [x] `install.ts` — BunShim object for `globalThis.Bun`
- [x] `sqlite.ts` — Bun `Database` polyfill over `node:sqlite`
- [x] `password.ts` — fixed Uint8Array type
- [ ] Commit pending changes (from-bun.ts, password.ts, sqlite.ts)
- [ ] Add README.md
