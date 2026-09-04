use crate::{
    api,
    chart::{Chart, ChartMarker, MarkerKind, Series},
};
use fund_types::{BacktestInput, BacktestReport, FundInfo, StrategyInfo};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn BacktestPage() -> impl IntoView {
    let code = RwSignal::new(String::new());
    let strategy = RwSignal::new(String::from("buy_hold"));
    let initial = RwSignal::new(1000.0f64);
    let dca_amount = RwSignal::new(100.0f64);
    let dca_interval = RwSignal::new(7u64);
    let no_rules = RwSignal::new(false);

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
    let running = RwSignal::new(false);

    let on_run = move |_| {
        let input = BacktestInput {
            code: code.get_untracked(),
            strategy: strategy.get_untracked(),
            initial: initial.get_untracked(),
            dca_amount: dca_amount.get_untracked(),
            dca_interval: dca_interval.get_untracked(),
            from: None,
            to: None,
            no_rules: no_rules.get_untracked(),
        };
        running.set(true);
        let future = api::run_backtest(input);
        spawn_local(async move {
            match future.await {
                Ok(r) => report.set(Some(r)),
                Err(e) => {
                    leptos::logging::error!("backtest failed: {e}");
                }
            }
            running.set(false);
        });
    };

    view! {
        <h2>"Backtest"</h2>
        <form>
            <label for="fund-select">"Fund"</label>
            <select
                id="fund-select"
                name="fund-select"
                prop:value=code
                on:change=move |e| code.set(event_target_value(&e))
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
            <label for="no_rules">
                <input id="no_rules" name="no_rules" type="checkbox" prop:checked=no_rules on:change=move |e| {
                    no_rules.set(event_target_checked(&e));
                } />
                "Ignore stored fee rules"
            </label>
            <button type="button" on:click=on_run disabled=move || running.get()>
                {move || if running.get() { "Running..." } else { "Run" }}
            </button>
        </form>

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
                            series=vec![Series {
                                points: report.curve.clone(),
                                color: "#2b6cb0",
                                name: "equity",
                                markers: vec![],
                            }]
                        />
                    </section>
                    <section>
                        <h3>"NAV"</h3>
                        <Chart series=build_nav_series(&report)/>
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
        markers,
    }]
}
