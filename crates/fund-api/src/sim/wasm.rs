use crate::{
    error::Result,
    rules::Fifo,
    sim::{
        event::{Event, Order, TransactionKind as EngineTransactionKind},
        fees::FeeService,
        strategy::{SimContext, Strategy},
    },
};
use chrono::NaiveDate;
use std::path::Path;
use std::sync::Arc;
use wasmtime::{
    Engine, Store,
    component::{Component, HasSelf, Linker, ResourceTable},
};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    world: "fund-strategy",
    path: "../../wit/strategy.wit",
});

pub struct HostWasi {
    wasi: WasiCtx,
    table: ResourceTable,
    fee_service: Arc<FeeService>,
}

impl WasiView for HostWasi {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// The `fees` interface the platform exposes to strategies.
impl fund::strategy::fees::Host for HostWasi {
    fn redeem_fee(&mut self, shares: f64) -> f64 {
        self.fee_service.redeem_fee(shares)
    }

    fn subscribe_fee(&mut self, amount: f64) -> f64 {
        self.fee_service.subscribe_fee(amount)
    }
}

pub struct WasmStrategy {
    name: String,
    store: Store<HostWasi>,
    world: FundStrategy,
}

impl WasmStrategy {
    pub fn embedded(
        bytes: &'static [u8],
        name: String,
        config: &str,
        fee_service: Arc<FeeService>,
    ) -> Result<Self> {
        let mut strategy = Self::from_binary(bytes, name, fee_service)?;
        strategy.init(config)?;
        Ok(strategy)
    }

    pub fn from_file(
        path: &Path,
        name: String,
        config: &str,
        fee_service: Arc<FeeService>,
    ) -> Result<Self> {
        let engine = Engine::default();
        let component = Component::from_file(&engine, path)?;
        let mut strategy = Self::from_component(engine, component, name, fee_service)?;
        strategy.init(config)?;
        Ok(strategy)
    }

    /// Describe a component's metadata (name + description + config schema)
    /// without initializing it.
    pub fn metadata(bytes: &[u8]) -> Result<(String, String, String)> {
        let engine = Engine::default();
        let component = Component::from_binary(&engine, bytes)?;
        let mut strategy =
            Self::from_component(engine, component, "meta".to_string(), default_fee_service())?;
        let name = strategy
            .world
            .fund_strategy_trader()
            .call_name(&mut strategy.store)?;
        let description = strategy
            .world
            .fund_strategy_trader()
            .call_description(&mut strategy.store)?;
        let schema = strategy
            .world
            .fund_strategy_trader()
            .call_config_schema(&mut strategy.store)?;
        Ok((name, description, schema))
    }

    fn from_binary(bytes: &[u8], name: String, fee_service: Arc<FeeService>) -> Result<Self> {
        let engine = Engine::default();
        let component = Component::from_binary(&engine, bytes)?;
        Self::from_component(engine, component, name, fee_service)
    }

    fn from_component(
        engine: Engine,
        component: Component,
        name: String,
        fee_service: Arc<FeeService>,
    ) -> Result<Self> {
        let mut linker = Linker::<HostWasi>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        FundStrategy::add_to_linker::<_, HasSelf<_>>(&mut linker, |state: &mut HostWasi| state)?;
        let mut store = Store::new(
            &engine,
            HostWasi {
                wasi: WasiCtxBuilder::new().build(),
                table: ResourceTable::new(),
                fee_service,
            },
        );
        let world = FundStrategy::instantiate(&mut store, &component, &linker)?;
        Ok(Self { name, store, world })
    }

    fn init(&mut self, config: &str) -> Result<()> {
        let result = self
            .world
            .fund_strategy_trader()
            .call_init(&mut self.store, config)?;
        result.map_err(crate::error::Error::Wasm)
    }
}

fn default_fee_service() -> Arc<FeeService> {
    let today = NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid date");
    Arc::new(FeeService::new(Box::new(Fifo::new(vec![], vec![])), today))
}

impl Strategy for WasmStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_event(&mut self, event: &Event, _ctx: &mut SimContext) -> Vec<Order> {
        let Some(wit_event) = map_event(event) else {
            return Vec::new();
        };
        let orders = self
            .world
            .fund_strategy_trader()
            .call_on_event(&mut self.store, &wit_event)
            .unwrap_or_default();
        orders
            .into_iter()
            .map(|order| match order {
                exports::fund::strategy::trader::Order::Invest(amount) => Order::Invest { amount },
                exports::fund::strategy::trader::Order::Redeem(shares) => Order::Redeem { shares },
            })
            .collect()
    }
}

fn map_event(event: &Event) -> Option<exports::fund::strategy::trader::Event> {
    use exports::fund::strategy::trader::{
        Event as WitEvent, InvestExecuted, NavUpdate, OrderExecuted, RedeemExecuted,
        TransactionKind,
    };
    match event {
        Event::NavUpdate {
            date,
            unit_nav,
            accum_nav,
        } => Some(WitEvent::NavUpdate(NavUpdate {
            date: date.to_string(),
            unit_nav: *unit_nav,
            accum_nav: *accum_nav,
        })),
        Event::OrderExecuted { date, transaction } => {
            let kind = match &transaction.kind {
                EngineTransactionKind::Invest {
                    amount,
                    shares,
                    fee,
                } => TransactionKind::Invest(InvestExecuted {
                    amount: *amount,
                    shares: *shares,
                    fee: *fee,
                }),
                EngineTransactionKind::Redeem { shares, money, fee } => {
                    TransactionKind::Redeem(RedeemExecuted {
                        shares: *shares,
                        money: *money,
                        fee: *fee,
                    })
                }
            };
            Some(WitEvent::OrderExecuted(OrderExecuted {
                date: date.to_string(),
                unit_nav: transaction.unit_nav,
                kind,
            }))
        }
        Event::DayStart(_) | Event::DayEnd(_) => None,
    }
}
