# fund

A web-based backtesting application for mutual fund trading strategies,
written in Rust with Leptos (SSR + client-side hydration).

## Features

- Web UI to fetch real daily NAV history from Eastmoney and store it in SQLite.
- Backtest trading strategies against historical NAV data from the browser.
- Event-driven simulation engine with T+1 (next-day NAV) order execution.
- Pluggable fee rules (e.g. FIFO redemption fee by holding period).
- **WASM-based strategies**: every strategy, including the bundled `buy_hold`
  and `dca`, is a WebAssembly component (WIT interface) that is built into the
  binary. Third-party strategies can be loaded from a `strategy.toml`.
- Interactive equity-curve chart (zoom / pan / hover) rendered as SVG.

## Running

```sh
cargo run                      # starts the server
# FUND_DB=/path/to/fund.db     # database path (default: ./fund.db)
# FUND_ADDR=127.0.0.1:8080     # bind address (default: 127.0.0.1:8080)
```

Open http://127.0.0.1:8080 in a browser.

The server renders the app server-side (SSR); the client-side UI is compiled to
WebAssembly (`wasm32-unknown-unknown`) by `build.rs` and embedded into the
binary, so no extra tooling is needed to run.

## Pages

- **Funds**: list funds already in the database, add a fund by code (e.g.
  `110022`) with the *Fetch* button, or refresh one with *Update*.
- **Backtest**: pick a fund and strategy, configure parameters, run the
  backtest, and view the report summary plus an interactive equity-curve chart.

## Strategies

- `buy_hold`: invest `--initial`-equivalent amount once and hold.
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

### Writing a strategy

A strategy implements the interface in `wit/strategy.wit`: `init(config)` is
called once with the serialized `[params]` table, then `on-event(event)` is
called for each `nav-update` and `order-executed` event and returns a list of
orders. Strategies keep their own state (including their recorded holdings)
inside the guest.

## Fees

The backtest applies a FIFO redemption fee rule (fee rate depends on holding
period) and a tiered subscription fee rule. Rules are defined in `src/rules.rs`
and can be extended by implementing the `Rule` trait.

## Building the bundled strategies

The bundled WASM strategies are built automatically by `build.rs`, which runs
`cargo build --target wasm32-wasip2` for each guest and embeds the resulting
components into the binary. The client-side UI is built for
`wasm32-unknown-unknown` and processed with `wasm-bindgen`. Building requires
the `wasm32-wasip2` and `wasm32-unknown-unknown` Rust targets
(`rustup target add wasm32-wasip2 wasm32-unknown-unknown`) and `wasm-bindgen`.
