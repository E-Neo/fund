use crate::web::server::{fetch_fund, list_funds, update_fund};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn FundsPage() -> impl IntoView {
    let funds = Resource::new(|| (), |_| list_funds());
    let code = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());

    let on_fetch = move |_| {
        let code = code.get_untracked();
        if code.is_empty() {
            return;
        }
        message.set(format!("fetching {code}..."));
        let future = fetch_fund(code.clone());
        spawn_local(async move {
            match future.await {
                Ok(fund) => {
                    message.set(format!("fetched {}", fund.name));
                    funds.refetch();
                }
                Err(err) => message.set(format!("failed: {err}")),
            }
        });
    };

    let on_update = move |fund_code: String| {
        message.set(format!("updating {fund_code}..."));
        let future = update_fund(fund_code.clone());
        spawn_local(async move {
            match future.await {
                Ok(fund) => {
                    message.set(format!("updated {}", fund.name));
                    funds.refetch();
                }
                Err(err) => message.set(format!("failed: {err}")),
            }
        });
    };

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
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || {
                funds.get().map(|result| match result {
                    Ok(list) => {
                        view! {
                            <table>
                                <thead>
                                    <tr><th>"Code"</th><th>"Name"</th><th></th></tr>
                                </thead>
                                <tbody>
                                    {list.into_iter().map(|fund| {
                                        let code = fund.code.clone();
                                        view! {
                                            <tr>
                                                <td>{fund.code.clone()}</td>
                                                <td>{fund.name}</td>
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
                    Err(err) => view! { <p>{format!("Error: {err}")}</p> }.into_any(),
                })
            }}
        </Suspense>
    }
}
