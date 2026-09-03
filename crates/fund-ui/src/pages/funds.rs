use crate::api;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

#[component]
pub fn FundsPage() -> impl IntoView {
    let funds = RwSignal::new(None::<Result<Vec<fund_types::FundInfo>, String>>);
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

    view! {
        <h2>"Funds"</h2>
        <div class="fetch-row">
            <input
                type="text"
                placeholder="fund code, e.g. 110022"
                prop:value=code
                on:input=move |e| code.set(event_target_value(&e))
            />
            <button on:click=on_fetch>"Fetch"</button>
        </div>
        <p>{move || message.get()}</p>
        <div>
            {move || match funds.get() {
                Some(Ok(list)) => {
                    let on_update = on_update.clone();
                    view! {
                        <table>
                            <thead>
                                <tr><th>"Code"</th><th>"Name"</th><th></th></tr>
                            </thead>
                            <tbody>
                                {list.iter().map(move |fund| {
                                    let code = fund.code.clone();
                                    let on_update = on_update.clone();
                                    view! {
                                        <tr>
                                            <td>{fund.code.clone()}</td>
                                            <td>{fund.name.clone()}</td>
                                            <td>
                                                <button on:click=move |_| on_update(code.clone())>
                                                    "Update"
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
    }
}
