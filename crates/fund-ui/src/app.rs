use crate::pages::{backtest::BacktestPage, funds::FundsPage};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    let open = RwSignal::new(false);
    let close = move |_| {
        open.set(false);
    };

    view! {
        <main class="container">
            <h1>"Fund Backtester"</h1>
            <Router>
                <nav class="topnav">
                    <button
                        class="nav-toggle"
                        aria-label="Menu"
                        aria-expanded=move || open.get()
                        on:click=move |_| open.update(|o| *o = !*o)
                    >
                        {move || if open.get() { "✕" } else { "☰" }}
                    </button>
                    <div class=move || if open.get() { "nav-links open" } else { "nav-links" }>
                        <a href="/" on:click=close>"Funds"</a>
                        <a href="/backtest" on:click=close>"Backtest"</a>
                    </div>
                </nav>
                <Routes fallback=|| view! { <p>"Not found"</p> }>
                    <Route path=path!("/") view=FundsPage/>
                    <Route path=path!("/backtest") view=BacktestPage/>
                </Routes>
            </Router>
        </main>
    }
}
