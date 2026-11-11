# Signup wizard prototype (throwaway)

A clickable, phone-sized version of the six-step signup wizard
(`docs/design/01-signup-wizard-spec.md`), for trying the flow and the mobile look. It is **not**
the real front end; that is still to be written.

It runs entirely in the browser, with no server and no database. The backend's pure rules modules
(`model`, `content`, `dice`, `derived`, `allocator`, `projection`) are compiled to WebAssembly
unchanged, straight from `backend/src/` by path. `wasm/src/lib.rs` mirrors the request handling in
`backend/src/routes/` (patch validation, dice ceremonies, declared dice, finalize, career/hobby
cards). Given the same answers, it produces a byte-identical sheet to the live API.

## Build and open

Needs Rust (the script adds the `wasm32-unknown-unknown` target if missing) and `python3`.

```bash
./prototype/build.sh
open prototype/web/dist/index.html        # macOS; or just double-click the file
```

The output is one self-contained HTML file with the wasm inlined, so it works from `file://`.
Drafts and sealed investigators are kept in that browser's localStorage only.

Smoke test the wasm build on its own (Node 18+):

```bash
node prototype/wasm/smoke-test.mjs
```

## Layout

```
prototype/
├── build.sh                    # cargo build (wasm) → inline into web/dist/index.html
├── wasm/
│   ├── Cargo.toml              # standalone crate, own [workspace]
│   ├── src/lib.rs              # #[path] includes of backend/src + route mirror + C ABI
│   └── smoke-test.mjs          # full wizard run in Node, no browser
└── web/
    └── index.template.html     # the UI; __WASM_B64__ is replaced at build time
```

`wasm/target/` and `web/dist/` are gitignored.

## Keeping it in sync

- Rules changes in `backend/src/{model,content,dice,derived,allocator,projection}.rs` and
  `backend/content/starter_pack.json` are picked up by re-running `build.sh`.
- Changes to `backend/src/routes/*.rs` are **not**. Those handlers depend on axum and sqlx, so
  `wasm/src/lib.rs` carries a copy of their logic. Update it by hand if a route's validation
  changes.
- If one of the six pure modules ever imports `db`, `axum` or `sqlx`, this crate stops compiling.
  That is a useful alarm: those modules are meant to stay pure.

## Known gaps

- Backstory fate tables are a placeholder; the table content isn't authored yet (spec § 8).
- No accounts or sync: everything lives in one browser.
- The fan-policy notice is under "About this page" on the Identity and Sheet steps.

## Running the real backend locally instead

See `backend/README.md`. In short:

```bash
cd backend
docker compose up -d
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/coc_phoenix
cargo run          # migrations and the starter content pack apply on boot
curl localhost:8080/health
```
