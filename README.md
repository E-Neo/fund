# fund

A web-based backtesting application for mutual fund trading strategies,
written in Rust with Leptos (SSR + client-side hydration).

## Workspace layout

```
crates/
  fund-types/        shared serde DTOs (FundInfo, BacktestInput, ...)
  fund-api/          domain logic + axum REST API under /api
  fund-ui/           Leptos web app (SSR + hydration) + server binary
  fund-strategies/   WASM strategy guest component
wit/strategy.wit     WIT contract for strategies
```

- `fund-api` owns the domain: data fetching (Eastmoney), fee rules, the
  event-driven simulation engine, report generation, and a plain axum Router
  exposing `GET/POST /api/*` endpoints.
- `fund-ui` is the Leptos application. The server binary mounts the `fund-api`
  router, serves the server-rendered pages, and serves the client-side WASM +
  assets. In the browser it talks to `/api` over plain `fetch`.
- Strategies are compiled to WASM components; the guest is built by
  `fund-api`'s build script and embedded into the binary.

## Running

```sh
cargo run -- --home <DIR>
```

`--home` is required. `<DIR>` is the home directory containing the
configuration and database:

- `config.toml` — required server configuration:
  ```toml
  [server]
  ip = "127.0.0.1"
  port = 8080
  ```
  Missing file → startup fails with an error.
- `db.sqlite3` — the SQLite database (created if missing).

Open http://127.0.0.1:8080 in a browser.

## Pages

- **Funds**: list funds already in the database, add a fund by code (e.g.
  `110022`) with the *Fetch* button, or refresh one with *Update*.
- **Backtest**: pick a fund and strategy, configure parameters, run the
  backtest, and view the report summary plus an interactive equity-curve chart.

## REST API

Handled by `fund-api` via axum:

```
GET  /api/funds                      list cached funds
POST /api/funds/{code}/fetch         fetch fund + fee rules from Eastmoney
POST /api/funds/{code}/update        incremental NAV update
GET  /api/funds/{code}/navs          NAV history
GET  /api/funds/{code}/rules         fee tiers
GET  /api/strategies                 available strategies
POST /api/backtest                   run a backtest (BacktestInput -> BacktestReport)
```

## Strategies

- `buy_hold`: invest an initial amount once and hold.
- `dca`: invest a fixed amount on a regular schedule.

Strategies are compiled to WebAssembly components and embedded into the
binary. Bundled strategy sources live in `crates/fund-strategies/`, where the
builtin `buy_hold` and `dca` strategies are selected via the `strategy` key in
the params. A custom strategy is described by a TOML file pointing at a
component:

```toml
module = "/path/to/my-strategy.wasm"

[params]
amount = 100.0
```

A strategy implements the interface in `wit/strategy.wit`: `init(config)` is
called once with the serialized `[params]` table, then `on-event(event)` is
called for each `nav-update` and `order-executed` event and returns a list of
orders. Strategies keep their own state (including their recorded holdings)
inside the guest.

## Fees

The backtest applies a FIFO redemption fee rule (fee rate depends on holding
period) and a tiered subscription fee rule. Rules are defined in
`crates/fund-api/src/rules.rs` and can be extended by implementing the `Rule`
trait.

## Building

- `cargo build --workspace` builds `fund-api`, `fund-ui` (server + client
  WASM) and runs the embedded build scripts.
- The client WASM is built by `fund-ui`'s build script
  (`wasm32-unknown-unknown`, processed with `wasm-bindgen`) and embedded into
  the binary, so no extra tooling is needed at runtime.
- Building requires the `wasm32-wasip2` and `wasm32-unknown-unknown` Rust
  targets (`rustup target add wasm32-wasip2 wasm32-unknown-unknown`) and
  `wasm-bindgen`.
