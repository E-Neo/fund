use crate::{
    error::{Error, Result},
    sim::{
        event::{Event, Order, Transaction},
        state::PortfolioState,
        wasm::WasmStrategy,
    },
};
use chrono::NaiveDate;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum StrategyArg {
    Bundled(String),
    File(PathBuf),
}

pub struct SimContext<'a> {
    pub date: NaiveDate,
    pub unit_nav: f64,
    pub accum_nav: f64,
    pub state: &'a PortfolioState,
    pub transactions: &'a [Transaction],
}

pub trait Strategy {
    fn name(&self) -> &str;
    fn on_event(&mut self, event: &Event, ctx: &mut SimContext) -> Vec<Order>;
}

const FUND_STRATEGIES_WASM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/fund_strategies.wasm"));

pub fn load(arg: &StrategyArg, dca_amount: f64, dca_interval: u64) -> Result<Box<dyn Strategy>> {
    match arg {
        StrategyArg::Bundled(name) => {
            let config = match name.as_str() {
                "dca" => serde_json::to_string(&serde_json::json!({
                    "strategy": "dca",
                    "amount": dca_amount,
                    "interval": dca_interval,
                }))
                .map_err(|err| Error::Parse(format!("failed to serialize config: {err}")))?,
                other => return Err(Error::UnknownStrategy(other.to_string())),
            };
            embedded(name, &config)
        }
        StrategyArg::File(path) => load_plugin(path),
    }
}

pub fn embedded(name: &str, config: &str) -> Result<Box<dyn Strategy>> {
    match name {
        "dca" => Ok(Box::new(WasmStrategy::embedded(
            FUND_STRATEGIES_WASM,
            name.to_string(),
            config,
        )?)),
        other => Err(Error::UnknownStrategy(other.to_string())),
    }
}

fn load_plugin(path: &Path) -> Result<Box<dyn Strategy>> {
    let text = std::fs::read_to_string(path)?;
    let parsed: serde_json::Value = serde_json::from_str(&text)
        .map_err(|err| Error::Parse(format!("invalid strategy json {}: {err}", path.display())))?;
    let module = parsed
        .get("module")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Parse("missing `module` in strategy json".to_string()))?;
    let module_path = path
        .parent()
        .map(|dir| dir.join(module))
        .unwrap_or_else(|| Path::new(module).to_path_buf());
    let params = parsed
        .get("params")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let config = serde_json::to_string(&params)
        .map_err(|err| Error::Parse(format!("failed to serialize params: {err}")))?;
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "plugin".to_string());
    WasmStrategy::from_file(&module_path, name, &config).map(|s| Box::new(s) as Box<dyn Strategy>)
}

pub fn names() -> [&'static str; 1] {
    ["dca"]
}
