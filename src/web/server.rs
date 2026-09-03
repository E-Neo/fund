use crate::web::types::{BacktestInput, BacktestReport, FeeTier, FundInfo, NavPoint, StrategyInfo};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::{
    fees::FeeRule,
    report, rules,
    sim::{
        engine,
        strategy::{self, StrategyArg},
    },
    web::state,
};

#[cfg(feature = "ssr")]
use crate::web::types::CurvePoint;

#[server(ListFunds, "/api/funds")]
pub async fn list_funds() -> Result<Vec<FundInfo>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let funds = crate::db::list_funds(state::pool()).await?;
        Ok(funds
            .into_iter()
            .map(|(code, name)| FundInfo { code, name })
            .collect())
    }
    #[cfg(not(feature = "ssr"))]
    {
        unreachable!()
    }
}

#[server(FetchFund, "/api/funds/fetch")]
pub async fn fetch_fund(code: String) -> Result<FundInfo, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let fund = crate::eastmoney::fetch_fund(&code).await?;
        crate::db::upsert_fund(state::pool(), &fund).await?;
        let fee_rule = crate::eastmoney::fetch_fees(&code).await?;
        crate::db::upsert_rules(state::pool(), &code, &fee_rule).await?;
        Ok(FundInfo {
            code,
            name: fund.name,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        unreachable!()
    }
}

#[server(UpdateFund, "/api/funds/update")]
pub async fn update_fund(code: String) -> Result<FundInfo, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use chrono::{Duration, Local};
        match crate::db::max_nav_date(state::pool(), &code).await? {
            None => {
                let fund = crate::eastmoney::fetch_fund(&code).await?;
                crate::db::upsert_fund(state::pool(), &fund).await?;
                Ok(FundInfo {
                    code,
                    name: fund.name,
                })
            }
            Some(last) => {
                let name = crate::db::fund_name(state::pool(), &code)
                    .await?
                    .unwrap_or_else(|| "unknown".to_string());
                let today = Local::now().date_naive();
                let navs =
                    crate::eastmoney::fetch_nav_range(&code, last + Duration::days(1), today)
                        .await?;
                if navs.is_empty() {
                    Ok(FundInfo { code, name })
                } else {
                    let fund = crate::eastmoney::Fund { code, name, navs };
                    crate::db::upsert_fund(state::pool(), &fund).await?;
                    Ok(FundInfo {
                        code: fund.code,
                        name: fund.name,
                    })
                }
            }
        }
    }
    #[cfg(not(feature = "ssr"))]
    {
        unreachable!()
    }
}

#[server(FundNavs, "/api/funds/navs")]
pub async fn fund_navs(code: String) -> Result<Vec<NavPoint>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let navs = crate::db::load_navs(state::pool(), &code).await?;
        Ok(navs
            .into_iter()
            .map(|n| NavPoint {
                date: n.date.to_string(),
                unit_nav: n.unit_nav,
                accum_nav: n.accum_nav,
                daily_return: n.daily_return,
            })
            .collect())
    }
    #[cfg(not(feature = "ssr"))]
    {
        unreachable!()
    }
}

#[server(FundRules, "/api/funds/rules")]
pub async fn fund_rules(code: String) -> Result<Vec<FeeTier>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let rule = crate::db::load_rules(state::pool(), &code).await?;
        let mut out = Vec::new();
        if let Some(rule) = rule {
            for tier in rule.subscribe {
                out.push(FeeTier {
                    rule_type: "subscribe".to_string(),
                    lower_bound: tier.lower_bound,
                    rate: tier.rate,
                    fee_kind: format!("{:?}", tier.kind),
                });
            }
            for tier in rule.redeem {
                out.push(FeeTier {
                    rule_type: "redeem".to_string(),
                    lower_bound: tier.lower_bound,
                    rate: tier.rate,
                    fee_kind: format!("{:?}", tier.kind),
                });
            }
        }
        Ok(out)
    }
    #[cfg(not(feature = "ssr"))]
    {
        unreachable!()
    }
}

#[server(ListStrategies, "/api/strategies")]
pub async fn list_strategies() -> Result<Vec<StrategyInfo>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        Ok(strategy::names()
            .iter()
            .map(|name| StrategyInfo {
                name: (*name).to_string(),
                description: match *name {
                    "buy_hold" => "invest once and hold".to_string(),
                    "dca" => "invest a fixed amount on a regular schedule".to_string(),
                    _ => String::new(),
                },
            })
            .collect())
    }
    #[cfg(not(feature = "ssr"))]
    {
        unreachable!()
    }
}

#[server(RunBacktest, "/api/backtest")]
pub async fn run_backtest(input: BacktestInput) -> Result<BacktestReport, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let result = run_backtest_impl(&input).await?;
        Ok(result)
    }
    #[cfg(not(feature = "ssr"))]
    {
        unreachable!()
    }
}

#[cfg(feature = "ssr")]
async fn run_backtest_impl(input: &BacktestInput) -> crate::error::Result<BacktestReport> {
    use chrono::NaiveDate;
    let navs = crate::db::load_navs(state::pool(), &input.code).await?;
    let from = input
        .from
        .as_deref()
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()?;
    let to = input
        .to
        .as_deref()
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()?;
    let navs: Vec<_> = navs
        .into_iter()
        .filter(|nav| from.is_none_or(|f| nav.date >= f) && to.is_none_or(|t| nav.date <= t))
        .collect();
    if navs.is_empty() {
        return Err(crate::error::Error::NoData(input.code.clone()));
    }
    let start = navs.first().expect("non-empty").date;
    let end = navs.last().expect("non-empty").date;
    let days = navs.len();

    let stored = if input.no_rules {
        None
    } else {
        crate::db::load_rules(state::pool(), &input.code).await?
    };
    let fee_rule = stored.unwrap_or_else(|| FeeRule {
        subscribe: vec![],
        redeem: vec![],
    });
    let mut fee_rule = rules::Fifo::new(fee_rule.subscribe, fee_rule.redeem);
    let arg = StrategyArg::Bundled(input.strategy.clone());
    let mut strategy = strategy::load(&arg, input.initial, input.dca_amount, input.dca_interval)?;
    let result = engine::simulate(&navs, &mut fee_rule, strategy.as_mut())?;
    let report = report::build(start, end, days, &result);

    let curve = result
        .snapshots
        .iter()
        .map(|s| CurvePoint {
            date: s.date.to_string(),
            market_value: s.market_value(),
        })
        .collect();

    Ok(BacktestReport {
        start: report.start.to_string(),
        end: report.end.to_string(),
        days: report.days,
        transactions: report.transactions,
        total_invested: report.total_invested,
        total_redeemed: report.total_redeemed,
        final_holding_share: report.final_holding_share,
        final_market_value: report.final_market_value,
        profit: report.profit,
        total_return_pct: report.total_return_pct,
        max_drawdown_pct: report.max_drawdown_pct,
        curve,
    })
}
