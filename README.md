# fund

An event-driven backtesting CLI for mutual fund trading strategies.

## Features

- Fetch real daily NAV history from Eastmoney and store it in SQLite.
- Backtest trading strategies against historical NAV data.
- Event-driven simulation engine with T+1 (next-day NAV) order execution.
- Pluggable fee rules (e.g. FIFO redemption fee by holding period).

## Commands

```
fund fetch <CODE> --db <PATH>          Fetch daily NAV history into SQLite
fund backtest <CODE> --strategy <NAME> --db <PATH> [--from DATE] [--to DATE]
                 [--initial AMOUNT] [--dca-amount AMOUNT] [--dca-interval DAYS]
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

## Fees

The backtest applies a FIFO redemption fee rule (fee rate depends on holding
period) and a tiered subscription fee rule. Rules are defined in `src/rules.rs`
and can be extended by implementing the `Rule` trait.
