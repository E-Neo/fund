use crate::api;
use fund_types::{FundInfo, NavPoint};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

#[component]
pub fn FundsPage() -> impl IntoView {
    let funds = RwSignal::new(None::<Result<Vec<FundInfo>, String>>);
    let code = RwSignal::new(String::new());
    let search = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let navs = RwSignal::new(None::<Result<Vec<NavPoint>, String>>);
    let navs_code = RwSignal::new(None::<String>);

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

    let on_update = Arc::new(move |fund_code: String| {
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

    let on_navs = move |fund_code: String| {
        navs_code.set(Some(fund_code.clone()));
        navs.set(None);
        let future = api::fund_navs(fund_code);
        spawn_local(async move {
            navs.set(Some(future.await));
        });
    };

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
        <div class="fetch-row">
            <label for="fund-search">"Search cached funds"</label>
            <input
                id="fund-search"
                name="fund-search"
                type="search"
                placeholder="search by code or name"
                prop:value=search
                on:input=move |e| search.set(event_target_value(&e))
            />
            <select
                id="fund-select"
                name="fund-select"
                on:change=move |e| {
                    let value = event_target_value(&e);
                    if !value.is_empty() {
                        code.set(value);
                    }
                }
            >
                <option value="">"Select a fund..."</option>
                {move || match funds.get() {
                    Some(Ok(list)) => {
                        let query = search.get();
                        list.iter()
                            .filter(move |fund| {
                                query.is_empty()
                                    || fund.code.contains(&query)
                                    || fund.name.contains(&query)
                            })
                            .map(|fund| view! {
                                <option value=fund.code.clone()>
                                    {format!("{} ({})", fund.name, fund.code)}
                                </option>
                            })
                            .collect_view()
                            .into_any()
                    }
                    _ => view! { <option>"Loading..."</option> }.into_any(),
                }}
            </select>
        </div>
        <p>{move || message.get()}</p>
        <div>
            {move || match funds.get() {
                Some(Ok(list)) => {
                    let on_update = on_update.clone();
                    view! {
                        <table>
                            <thead>
                                <tr><th>"Code"</th><th>"Name"</th><th></th><th></th></tr>
                            </thead>
                            <tbody>
                                {list.iter().map(move |fund| {
                                    let code = fund.code.clone();
                                    let update_code = code.clone();
                                    let navs_code = code.clone();
                                    let on_update = on_update.clone();
                                    view! {
                                        <tr>
                                            <td>{fund.code.clone()}</td>
                                            <td>{fund.name.clone()}</td>
                                            <td>
                                                <button on:click=move |_| on_update(update_code.clone())>
                                                    "Update"
                                                </button>
                                            </td>
                                            <td>
                                                <button on:click=move |_| on_navs(navs_code.clone())>
                                                    "Navs"
                                                </button>
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    }
                    .into_any()
                }
                Some(Err(err)) => view! { <p>{format!("Error: {err}")}</p> }.into_any(),
                None => view! { <p>"Loading..."</p> }.into_any(),
            }}
        </div>
        {move || navs_code.get().map(|fund_code| {
            view! {
                <section>
                    <h3>{format!("NAVs for {fund_code}")}</h3>
                    {match navs.get() {
                        Some(Ok(list)) => view! {
                            <div class="navs-scroll">
                                <table>
                                    <thead>
                                        <tr>
                                            <th>"Date"</th>
                                            <th>"Unit NAV"</th>
                                            <th>"Accum NAV"</th>
                                            <th>"Daily %"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.iter().rev().map(|nav| view! {
                                            <tr>
                                                <td>{nav.date.clone()}</td>
                                                <td>{format!("{:.4}", nav.unit_nav)}</td>
                                                <td>{format!("{:.4}", nav.accum_nav)}</td>
                                                <td>{nav.daily_return.map(|r| format!("{:.2}", r * 100.0)).unwrap_or_default()}</td>
                                            </tr>
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        }.into_any(),
                        Some(Err(err)) => view! { <p>{format!("Error: {err}")}</p> }.into_any(),
                        None => view! { <p>"Loading..."</p> }.into_any(),
                    }}
                </section>
            }
        })}
    }
}
