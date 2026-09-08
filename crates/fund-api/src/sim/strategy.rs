use crate::{
    error::{Error, Result},
    fees::FeeRule,
    sim::{
        event::{Event, Order, Transaction},
        oracle::Oracle,
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

/// Context needed to construct a strategy that can see the whole backtest
/// (e.g. the native oracle).
pub struct StrategyCtx<'a> {
    pub navs: &'a [crate::eastmoney::Nav],
    pub fee_rule: &'a FeeRule,
    pub capital: f64,
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

/// Embedded wasm strategy components.
const WASM_BLOBS: &[&[u8]] = &[include_bytes!(concat!(
    env!("OUT_DIR"),
    "/dollar_cost_averaging.wasm"
))];

/// A bundled strategy: metadata plus, for wasm components, the raw bytes.
struct Builtin {
    meta: StrategyMeta,
    bytes: Option<&'static [u8]>,
}

/// Metadata (name, description, config schema) for a bundled strategy. The
/// name is both the display name and the key used to select the strategy.
#[derive(Debug, Clone)]
pub struct StrategyMeta {
    pub name: String,
    pub description: String,
    pub schema: String,
}

fn builtins() -> &'static [Builtin] {
    static CACHE: OnceLock<Vec<Builtin>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut list = Vec::new();
        for bytes in WASM_BLOBS {
            if let Ok((name, description, schema)) = WasmStrategy::metadata(bytes) {
                list.push(Builtin {
                    meta: StrategyMeta {
                        name,
                        description,
                        schema,
                    },
                    bytes: Some(*bytes),
                });
            }
        }
        // Native strategies (see the future; not a wasm component).
        list.push(Builtin {
            meta: StrategyMeta {
                name: "Oracle".to_string(),
                description:
                    "sees the full history and future to maximize profit (virtual, not real)"
                        .to_string(),
                schema: r#"{"type":"object","properties":{}}"#.to_string(),
            },
            bytes: None,
        });
        list
    })
}

pub fn list() -> Vec<&'static StrategyMeta> {
    builtins().iter().map(|b| &b.meta).collect()
}

fn find_builtin(name: &str) -> Option<&'static Builtin> {
    builtins().iter().find(|b| b.meta.name == name)
}

pub fn load(
    arg: &StrategyArg,
    params: &serde_json::Value,
    ctx: &StrategyCtx,
) -> Result<Box<dyn Strategy>> {
    match arg {
        StrategyArg::Bundled(name) => {
            let builtin =
                find_builtin(name).ok_or_else(|| Error::UnknownStrategy(name.to_string()))?;
            match builtin.bytes {
                Some(bytes) => {
                    let config = serde_json::to_string(params).map_err(|err| {
                        Error::Parse(format!("failed to serialize config: {err}"))
                    })?;
                    Ok(Box::new(WasmStrategy::embedded(
                        bytes,
                        name.to_string(),
                        &config,
                    )?))
                }
                None => Ok(Box::new(Oracle::new(ctx.navs, ctx.fee_rule, ctx.capital))),
            }
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
