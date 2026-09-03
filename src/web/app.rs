use crate::web::pages::{backtest::BacktestPage, funds::FundsPage};
use leptos::config::LeptosOptions;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

/// The HTML shell wrapping the app, used for server-side rendering.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <HydrationScripts options=options.clone()/>
                <link rel="stylesheet" href="/pkg/styles.css"/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="Fund Backtester"/>
        <MetaTags/>
        <main class="container">
            <h1>"Fund Backtester"</h1>
            <Router>
                <nav>
                    <a href="/">"Funds"</a>
                    <a href="/backtest">"Backtest"</a>
                </nav>
                <Routes fallback=|| view! { <p>"Not found"</p> }>
                    <Route path=path!("/") view=FundsPage/>
                    <Route path=path!("/backtest") view=BacktestPage/>
                </Routes>
            </Router>
        </main>
    }
}
