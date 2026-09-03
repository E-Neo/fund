//! Client-side helpers to talk to the `/api` REST endpoints.
//!
//! On the browser (`wasm32`) these perform real HTTP requests through the
//! native `fetch` API. Elsewhere (the host build) they are never used and
//! return a "not on client" error.

use fund_types::{BacktestInput, BacktestReport, FundInfo, NavPoint, StrategyInfo};

pub type ApiResult<T> = Result<T, String>;

#[cfg(target_arch = "wasm32")]
async fn send<T: serde::de::DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<&str>,
) -> ApiResult<T> {
    use js_sys::Promise;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, Request, RequestInit, Response};

    let init = RequestInit::new();
    init.set_method(method);
    if let Some(body) = body {
        init.set_body(&wasm_bindgen::JsValue::from_str(body));
        let headers = Headers::new().map_err(js_err)?;
        headers
            .set("content-type", "application/json")
            .map_err(js_err)?;
        init.set_headers(&headers);
    }
    let request = Request::new_with_str_and_init(path, &init).map_err(js_err)?;
    let window = web_sys::window().expect("window should exist");
    let promise: Promise = window.fetch_with_request(&request);
    let resp: Response = JsFuture::from(promise)
        .await
        .map_err(js_err)?
        .dyn_into()
        .map_err(js_err)?;
    let status = resp.status();
    let text = JsFuture::from(resp.text().map_err(js_err)?)
        .await
        .map_err(js_err)?
        .as_string()
        .unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn js_err(err: wasm_bindgen::JsValue) -> String {
    err.as_string()
        .unwrap_or_else(|| "request failed".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn send<T>(_method: &str, _path: &str, _body: Option<&str>) -> ApiResult<T> {
    Err("api client is only available on the browser".to_string())
}

pub async fn list_funds() -> ApiResult<Vec<FundInfo>> {
    send("GET", "/api/funds", None).await
}

pub async fn fetch_fund(code: String) -> ApiResult<FundInfo> {
    send("POST", &format!("/api/funds/{code}/fetch"), None).await
}

pub async fn update_fund(code: String) -> ApiResult<FundInfo> {
    send("POST", &format!("/api/funds/{code}/update"), None).await
}

pub async fn list_strategies() -> ApiResult<Vec<StrategyInfo>> {
    send("GET", "/api/strategies", None).await
}

pub async fn run_backtest(input: BacktestInput) -> ApiResult<BacktestReport> {
    let body = serde_json::to_string(&input).map_err(|e| e.to_string())?;
    send("POST", "/api/backtest", Some(&body)).await
}

pub async fn fund_navs(code: String) -> ApiResult<Vec<NavPoint>> {
    send("GET", &format!("/api/funds/{code}/navs"), None).await
}
