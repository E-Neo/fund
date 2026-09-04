use crate::{
    api,
    chart::{Chart, Series},
};
use fund_types::{CurvePoint, FundInfo, NavPoint};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

#[component]
fn FundRow(fund: FundInfo, on_update: Arc<dyn Fn(String) + Send + Sync>) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let navs = RwSignal::new(None::<Result<Vec<NavPoint>, String>>);
    let fund_code = fund.code.clone();
    let toggle_code = fund_code.clone();
    let update_code = fund_code.clone();

    let toggle = move |_| {
        let now = expanded.get_untracked();
        expanded.set(!now);
        if !now {
            let future = api::fund_navs(toggle_code.clone());
            spawn_local(async move {
                navs.set(Some(future.await));
            });
        }
    };

    view! {
        <tr>
            <td>{fund_code.clone()}</td>
            <td>{fund.name.clone()}</td>
            <td>
                <button on:click=move |_| on_update(update_code.clone())>"Update"</button>
            </td>
            <td>
                <button on:click=toggle>
                    {move || if expanded.get() { "Hide chart" } else { "Show chart" }}
                </button>
            </td>
        </tr>
        {move || if expanded.get() {
            view! {
                <tr>
                    <td colspan="4">
                        {match navs.get() {
                            Some(Ok(list)) => view! {
                                <Chart series=vec![Series {
                                    points: list.iter().map(|n| CurvePoint {
                                        date: n.date.clone(),
                                        market_value: n.unit_nav,
                                    }).collect(),
                                    color: "#dd6b20",
                                    name: "nav",
                                    markers: vec![],
                                }]/>
                            }.into_any(),
                            Some(Err(err)) => view! { <p>{format!("Error: {err}")}</p> }.into_any(),
                            None => view! { <p>"Loading..."</p> }.into_any(),
                        }}
                    </td>
                </tr>
            }.into_any()
        } else {
            ().into_any()
        }}
    }
}

#[component]
pub fn FundsPage() -> impl IntoView {
    let funds = RwSignal::new(None::<Result<Vec<FundInfo>, String>>);
    let code = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());

    // Client-only fetch: in the wasm build this runs after hydration and
    // populates the table. In the SSR build the body is empty, so the page
    // renders its loading state and is populated on the client.
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let funds = funds;
            spawn_local(async move {
                funds.set(Some(api::list_funds().await));
            });
        }
    });

    let on_fetch = move |_| {
        let code = code.get_untracked();
        if code.is_empty() {
            return;
        }
        message.set(format!("fetching {code}..."));
        let future = api::fetch_fund(code);
        spawn_local(async move {
            match future.await {
                Ok(fund) => {
                    message.set(format!("fetched {}", fund.name));
                    funds.set(Some(api::list_funds().await));
                }
                Err(err) => message.set(format!("failed: {err}")),
            }
        });
    };

    let on_update: Arc<dyn Fn(String) + Send + Sync> = Arc::new(move |fund_code: String| {
        message.set(format!("updating {fund_code}..."));
        let future = api::update_fund(fund_code);
        spawn_local(async move {
            match future.await {
                Ok(fund) => {
                    message.set(format!("updated {}", fund.name));
                    funds.set(Some(api::list_funds().await));
                }
                Err(err) => message.set(format!("failed: {err}")),
            }
        });
    });

    view! {
        <h2>"Funds"</h2>
        <div class="fetch-row">
            <label for="fund-code">"Fetch a new fund by code"</label>
            <input
                id="fund-code"
                name="fund-code"
                type="text"
                placeholder="fund code, e.g. 110022"
                prop:value=code
                on:input=move |e| code.set(event_target_value(&e))
            />
            <button on:click=on_fetch>"Fetch"</button>
        </div>
        <p>{move || message.get()}</p>
        <div>
            {move || {
                let on_update = on_update.clone();
                match funds.get() {
                    Some(Ok(list)) => {
                        view! {
                            <table>
                                <thead>
                                    <tr><th>"Code"</th><th>"Name"</th><th></th><th></th></tr>
                                </thead>
                                <tbody>
                                    {list.iter().map(|fund| {
                                        let on_update = on_update.clone();
                                        view! {
                                            <FundRow fund={fund.clone()} on_update=on_update/>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        }
                        .into_any()
                    }
                    Some(Err(err)) => view! { <p>{format!("Error: {err}")}</p> }.into_any(),
                    None => view! { <p>"Loading..."</p> }.into_any(),
                }
            }}
        </div>
    }
}
