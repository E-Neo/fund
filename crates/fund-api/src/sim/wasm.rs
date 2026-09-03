use crate::{
    error::Result,
    sim::{
        event::{Event, Order, TransactionKind as EngineTransactionKind},
        strategy::{SimContext, Strategy},
    },
};
use std::path::Path;
use wasmtime::{
    Engine, Store,
    component::{Component, Linker, ResourceTable},
};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    world: "fund-strategy",
    path: "../../wit/strategy.wit",
});

pub struct HostWasi {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for HostWasi {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

pub struct WasmStrategy {
    name: String,
    store: Store<HostWasi>,
    world: FundStrategy,
}

impl WasmStrategy {
    pub fn embedded(bytes: &'static [u8], name: String, config: &str) -> Result<Self> {
        let mut strategy = Self::from_binary(bytes, name)?;
        strategy.init(config)?;
        Ok(strategy)
    }

    pub fn from_file(path: &Path, name: String, config: &str) -> Result<Self> {
        let engine = Engine::default();
        let component = Component::from_file(&engine, path)?;
        let mut strategy = Self::from_component(engine, component, name)?;
        strategy.init(config)?;
        Ok(strategy)
    }

    fn from_binary(bytes: &[u8], name: String) -> Result<Self> {
        let engine = Engine::default();
        let component = Component::from_binary(&engine, bytes)?;
        Self::from_component(engine, component, name)
    }

    fn from_component(engine: Engine, component: Component, name: String) -> Result<Self> {
        let mut linker = Linker::<HostWasi>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        let mut store = Store::new(
            &engine,
            HostWasi {
                wasi: WasiCtxBuilder::new().build(),
                table: ResourceTable::new(),
            },
        );
        let world = FundStrategy::instantiate(&mut store, &component, &linker)?;
        Ok(Self { name, store, world })
    }

    fn init(&mut self, config: &str) -> Result<()> {
        self.world
            .fund_strategy_strategy()
            .call_init(&mut self.store, config)?;
        Ok(())
    }
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
            .fund_strategy_strategy()
            .call_on_event(&mut self.store, &wit_event)
            .unwrap_or_default();
        orders
            .into_iter()
            .map(|order| match order {
                exports::fund::strategy::strategy::Order::Invest(amount) => {
                    Order::Invest { amount }
                }
                exports::fund::strategy::strategy::Order::Redeem(shares) => {
                    Order::Redeem { shares }
                }
            })
            .collect()
    }
}

fn map_event(event: &Event) -> Option<exports::fund::strategy::strategy::Event> {
    use exports::fund::strategy::strategy::{
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
