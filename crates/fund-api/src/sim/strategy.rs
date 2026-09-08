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
use std::sync::OnceLock;

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

/// Embedded strategy components, keyed by strategy name.
const BUILTINS: &[(&str, &[u8])] =
    &[("dca", include_bytes!(concat!(env!("OUT_DIR"), "/dca.wasm")))];

/// Metadata (name, description, config schema) for the bundled strategies,
/// loaded lazily from the embedded components.
#[derive(Debug, Clone)]
pub struct StrategyMeta {
    pub name: &'static str,
    pub description: String,
    pub schema: String,
}

fn bundled_bytes(name: &str) -> Option<&'static [u8]> {
    BUILTINS.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

fn metadata() -> &'static [StrategyMeta] {
    static CACHE: OnceLock<Vec<StrategyMeta>> = OnceLock::new();
    CACHE.get_or_init(|| {
        BUILTINS
            .iter()
            .filter_map(|(name, bytes)| {
                let (description, schema) = WasmStrategy::metadata(bytes).ok()?;
                Some(StrategyMeta {
                    name,
                    description,
                    schema,
                })
            })
            .collect()
    })
}

pub fn list() -> &'static [StrategyMeta] {
    metadata()
}

pub fn load(arg: &StrategyArg, params: &serde_json::Value) -> Result<Box<dyn Strategy>> {
    match arg {
        StrategyArg::Bundled(name) => {
            let bytes =
                bundled_bytes(name).ok_or_else(|| Error::UnknownStrategy(name.to_string()))?;
            let config = serde_json::to_string(params)
                .map_err(|err| Error::Parse(format!("failed to serialize config: {err}")))?;
            Ok(Box::new(WasmStrategy::embedded(
                bytes,
                name.to_string(),
                &config,
            )?))
        }
        StrategyArg::File(path) => load_plugin(path),
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
