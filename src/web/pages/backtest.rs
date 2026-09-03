use crate::web::{
    chart::Chart,
    server::{list_strategies, run_backtest},
    types::{BacktestInput, BacktestReport},
};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn BacktestPage() -> impl IntoView {
    let code = RwSignal::new(String::from("110022"));
    let strategy = RwSignal::new(String::from("buy_hold"));
    let initial = RwSignal::new(1000.0f64);
    let dca_amount = RwSignal::new(100.0f64);
    let dca_interval = RwSignal::new(7u64);
    let no_rules = RwSignal::new(false);

    let strategies = Resource::new(|| (), |_| list_strategies());

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
        let result = run_backtest(input);
        spawn_local(async move {
            match result.await {
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
            <label>"Fund code"</label>
            <input type="text" prop:value=code on:input=move |e| code.set(event_target_value(&e)) />
            <label>"Strategy"</label>
            <select prop:value=strategy on:change=move |e| strategy.set(event_target_value(&e))>
                <Suspense fallback=move || view! { <option>"Loading..."</option> }>
                    {move || {
                        strategies.get().map(|result| match result {
                            Ok(list) => list
                                .into_iter()
                                .map(|s| view! { <option value=s.name.clone()>{s.name.clone()}</option> })
                                .collect_view()
                                .into_any(),
                            Err(_) => view! { <option>"none"</option> }.into_any(),
                        })
                    }}
                </Suspense>
            </select>
            <label>"Initial amount"</label>
            <input type="number" prop:value=initial on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse() { initial.set(v) }
            } />
            <label>"DCA amount"</label>
            <input type="number" prop:value=dca_amount on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse() { dca_amount.set(v) }
            } />
            <label>"DCA interval (days)"</label>
            <input type="number" prop:value=dca_interval on:input=move |e| {
                if let Ok(v) = event_target_value(&e).parse() { dca_interval.set(v) }
            } />
            <label>
                <input type="checkbox" prop:checked=no_rules on:change=move |e| {
                    no_rules.set(event_target_checked(&e));
                } />
                "Ignore stored fee rules"
            </label>
            <button type="button" on:click=on_run disabled=move || running.get()>
                {move || if running.get() { "Running..." } else { "Run" }}
            </button>
        </form>

        <Suspense fallback=|| ()>
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
                        <Chart points=report.curve.clone()/>
                    </section>
                })
            }}
        </Suspense>
    }
}
