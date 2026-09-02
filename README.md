# fund

An event-driven backtesting CLI for mutual fund trading strategies.

## Features

- Fetch real daily NAV history from Eastmoney and store it in SQLite.
- Backtest trading strategies against historical NAV data.
- Event-driven simulation engine with T+1 (next-day NAV) order execution.
- Pluggable fee rules (e.g. FIFO redemption fee by holding period).
- **WASM-based strategies**: every strategy, including the bundled `buy_hold`
  and `dca`, is a WebAssembly component (WIT interface) that is built into the
  binary. Third-party strategies can be loaded from a `strategy.toml`.

## Commands

```
fund fetch <CODE> --db <PATH>          Fetch daily NAV history into SQLite
fund backtest <CODE> --strategy <NAME|TOML> --db <PATH> [--from DATE] [--to DATE]
                 [--initial AMOUNT] [--dca-amount AMOUNT] [--dca-interval DAYS]
fund update <CODE> --db <PATH>         Fetch NAV rows newer than the last stored date
fund list --db <PATH>                  List funds cached in the database
fund strategies                        List available strategies
```

## Example

```sh
cargo run -- fetch 110022 --db fund.db
cargo run -- backtest 110022 --strategy dca --db fund.db --dca-amount 500 --dca-interval 30
```

## Strategies

- `buy_hold`: invest `--initial` once and hold.
- `dca`: invest `--dca-amount` every `--dca-interval` trading days.

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
components into the binary. Building requires the `wasm32-wasip2` Rust target
(`rustup target add wasm32-wasip2`).
