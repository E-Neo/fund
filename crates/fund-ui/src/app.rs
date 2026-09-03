use crate::pages::{backtest::BacktestPage, funds::FundsPage};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    view! {
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
