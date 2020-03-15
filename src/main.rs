use fund::{
    crawler,
    rule::WeekRule,
    world::{Bank, Market, Repository, World},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = "001551";
    let market =
        Market::new(crawler::net_asset_value::get_history(code, "2020-03-02", "2020-03-06").await?);
    let bank = Bank::new(1000.);
    let repository = Repository::new(code, WeekRule::new());
    let mut world = World::new(bank, market, repository);
    world.update(fund::strategy::Plan::Buy(1000.))?;
    Ok(())
}
