use crate::{
    api,
    chart::{Chart, ChartMarker, MarkerKind, Series},
};
use fund_types::{BacktestInput, BacktestReport, FundInfo, NavRange, StrategyInfo};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn BacktestPage() -> impl IntoView {
    let code = RwSignal::new(String::new());
    let strategy = RwSignal::new(String::from("buy_hold"));
    let initial = RwSignal::new(1000.0f64);
    let dca_amount = RwSignal::new(100.0f64);
    let dca_interval = RwSignal::new(7u64);
    let from = RwSignal::new(String::new());
    let to = RwSignal::new(String::new());
    let range = RwSignal::new(None::<NavRange>);

    let strategies = RwSignal::new(None::<Result<Vec<StrategyInfo>, String>>);
    let funds = RwSignal::new(None::<Result<Vec<FundInfo>, String>>);

    // Client-only fetch of the strategy list (see FundsPage for the SSR note).
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let strategies = strategies;
            let funds = funds;
            spawn_local(async move {
                strategies.set(Some(api::list_strategies().await));
                funds.set(Some(api::list_funds().await));
            });
        }
    });

    let report = RwSignal::new(None::<BacktestReport>);
    let error = RwSignal::new(None::<String>);
    let running = RwSignal::new(false);

    let on_fund_change = move |e: leptos::ev::Event| {
        let value = event_target_value(&e);
        code.set(value.clone());
        if value.is_empty() {
            range.set(None);
            from.set(String::new());
            to.set(String::new());
            return;
        }
        let future = api::fund_range(value);
        spawn_local(async move {
            match future.await {
                Ok(r) => {
                    from.set(r.from.clone());
                    to.set(r.to.clone());
                    range.set(Some(r));
                }
                Err(err) => leptos::logging::error!("failed to load range: {err}"),
            }
        });
    };

    let on_run = move |_| {
        let from = {
            let v = from.get_untracked();
            if v.is_empty() { None } else { Some(v) }
        };
        let to = {
            let v = to.get_untracked();
            if v.is_empty() { None } else { Some(v) }
        };
        let input = BacktestInput {
            code: code.get_untracked(),
            strategy: strategy.get_untracked(),
            initial: initial.get_untracked(),
            dca_amount: dca_amount.get_untracked(),
            dca_interval: dca_interval.get_untracked(),
            from,
            to,
        };
        running.set(true);
        error.set(None);
        let future = api::run_backtest(input);
        spawn_local(async move {
            match future.await {
                Ok(r) => report.set(Some(r)),
                Err(e) => {
                    leptos::logging::error!("backtest failed: {e}");
                    error.set(Some(e));
                }
            }
            running.set(false);
        });
    };

    let range_min = move || range.get().map(|r| r.from).unwrap_or_default();
    let range_max = move || range.get().map(|r| r.to).unwrap_or_default();

    view! {
        <h2>"Backtest"</h2>
        <form>
            <label for="fund-select">"Fund"</label>
            <select
                id="fund-select"
                name="fund-select"
                prop:value=code
                on:change=on_fund_change
            >
                <option value="">"Select a fund..."</option>
                {move || match funds.get() {
                    Some(Ok(list)) => list
                        .iter()
                        .map(|fund| view! {
                            <option value=fund.code.clone()>
                                {format!("{} ({})", fund.name, fund.code)}
                            </option>
                        })
                        .collect_view()
                        .into_any(),
                    _ => view! { <option>"Loading..."</option> }.into_any(),
                }}
            </select>
            <label for="strategy">"Strategy"</label>
            <select id="strategy" name="strategy" prop:value=strategy on:change=move |e| strategy.set(event_target_value(&e))>
                {move || match strategies.get() {
                    Some(Ok(list)) => list
                        .iter()
                        .map(|s| view! { <option value=s.name.clone()>{s.name.clone()}</option> })
                        .collect_view()
                        .into_any(),
                    Some(Err(_)) => view! { <option>"none"</option> }.into_any(),
                    None => view! { <option>"Loading..."</option> }.into_any(),
                }}
            </select>
            <label for="from">"From"</label>
            <input
                id="from"
                name="from"
                type="date"
                prop:value=from
                min=range_min
                max=range_max
                on:input=move |e| from.set(event_target_value(&e))
            />
            <label for="to">"To"</label>
            <input
                id="to"
                name="to"
                type="date"
                prop:value=to
                min=range_min
                max=range_max
                on:input=move |e| to.set(event_target_value(&e))
            />
            <label for="initial">"Initial amount"</label>
            <input id="initial" name="initial" type="number" prop:value=initial on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse() { initial.set(v) }
            } />
            <label for="dca_amount">"DCA amount"</label>
            <input id="dca_amount" name="dca_amount" type="number" prop:value=dca_amount on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse() { dca_amount.set(v) }
            } />
            <label for="dca_interval">"DCA interval (days)"</label>
            <input id="dca_interval" name="dca_interval" type="number" prop:value=dca_interval on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse() { dca_interval.set(v) }
            } />
            <button type="button" on:click=on_run disabled=move || running.get()>
                {move || if running.get() { "Running..." } else { "Run" }}
            </button>
        </form>
        {move || error.get().map(|err| view! {
            <p class="error">{err}</p>
        })}

        <div>
            {move || {
                report.get().map(|report| view! {
                    <section>
                        <h3>"Report"</h3>
                        <pre>
                            {format!(
                                "Period: {} to {} ({} days)\nTransactions: {}\nInvested: {:.2}\nRedeemed: {:.2}\nFinal value: {:.2}\nProfit: {:.2}\nReturn: {:.2}%\nMax drawdown: {:.2}%",
                                report.start, report.end, report.days, report.transactions,
                                report.total_invested, report.total_redeemed,
                                report.final_market_value, report.profit,
                                report.total_return_pct, report.max_drawdown_pct,
                            )}
                        </pre>
                        <Chart
                            title="NAV".to_string()
                            y_label="NAV".to_string()
                            series=build_nav_series(&report)
                        />
                        <Chart
                            title="Equity curve".to_string()
                            y_label="Value".to_string()
                            series=vec![Series {
                                points: report.curve.clone(),
                                color: "#2b6cb0",
                                name: "equity",
                                decimals: 2,
                                markers: vec![],
                            }]
                        />
                        <Chart
                            title="Cumulative return".to_string()
                            y_label="%".to_string()
                            series=vec![Series {
                                points: report.return_curve.clone(),
                                color: "#38a169",
                                name: "return",
                                decimals: 2,
                                markers: vec![],
                            }]
                        />
                        <Chart
                            title="Invested vs Redeemed".to_string()
                            y_label="Amount".to_string()
                            series=vec![
                                Series {
                                    points: report.invested_curve.clone(),
                                    color: "#dd6b20",
                                    name: "invested",
                                    decimals: 2,
                                    markers: vec![],
                                },
                                Series {
                                    points: report.redeemed_curve.clone(),
                                    color: "#2b6cb0",
                                    name: "redeemed",
                                    decimals: 2,
                                    markers: vec![],
                                },
                            ]
                        />
                        <Chart
                            title="Drawdown".to_string()
                            y_label="%".to_string()
                            series=vec![Series {
                                points: report.drawdown_curve.clone(),
                                color: "#d63a3a",
                                name: "drawdown",
                                decimals: 2,
                                markers: vec![],
                            }]
                        />
                    </section>
                })
            }}
        </div>
    }
}

fn build_nav_series(report: &BacktestReport) -> Vec<Series> {
    let markers = report
        .markers
        .iter()
        .filter_map(|m| {
            let index = report.nav_curve.iter().position(|p| p.date == m.date)?;
            Some(ChartMarker {
                index,
                kind: if m.kind == "buy" {
                    MarkerKind::Buy
                } else {
                    MarkerKind::Sell
                },
            })
        })
        .collect();
    vec![Series {
        points: report.nav_curve.clone(),
        color: "#dd6b20",
        name: "nav",
        decimals: 4,
        markers,
    }]
}
