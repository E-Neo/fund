use crate::{
    config,
    fees::FeeRule,
    report, rules,
    sim::strategy::{self, StrategyArg},
};
use axum::{
    Json, Router,
    extract::Path,
    routing::{get, post},
};
use fund_types::{
    BacktestInput, BacktestMarker, BacktestReport, CurvePoint, FeeTier, FundInfo, NavPoint,
    NavRange, StrategyInfo,
};

/// Build the REST API router.
pub fn router() -> Router {
    Router::new()
        .route("/api/funds", get(list_funds))
        .route("/api/funds/{code}/fetch", post(fetch_fund))
        .route("/api/funds/{code}/update", post(update_fund))
        .route("/api/funds/{code}/navs", get(fund_navs))
        .route("/api/funds/{code}/range", get(fund_range))
        .route("/api/funds/{code}/rules", get(fund_rules))
        .route("/api/strategies", get(list_strategies))
        .route("/api/backtest", post(run_backtest))
}

type ApiError = (axum::http::StatusCode, String);

fn api_err<E: Into<crate::error::Error>>(err: E) -> ApiError {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.into().to_string(),
    )
}

async fn list_funds() -> Result<Json<Vec<FundInfo>>, ApiError> {
    let funds = crate::db::list_funds(config::pool())
        .await
        .map_err(api_err)?;
    Ok(Json(
        funds
            .into_iter()
            .map(|(code, name)| FundInfo { code, name })
            .collect(),
    ))
}

async fn fetch_fund(Path(code): Path<String>) -> Result<Json<FundInfo>, ApiError> {
    let fund = crate::eastmoney::fetch_fund(&code).await.map_err(api_err)?;
    crate::db::upsert_fund(config::pool(), &fund)
        .await
        .map_err(api_err)?;
    let fee_rule = crate::eastmoney::fetch_fees(&code).await.map_err(api_err)?;
    crate::db::upsert_rules(config::pool(), &code, &fee_rule)
        .await
        .map_err(api_err)?;
    Ok(Json(FundInfo {
        code,
        name: fund.name,
    }))
}

async fn update_fund(Path(code): Path<String>) -> Result<Json<FundInfo>, ApiError> {
    use chrono::{Duration, Local};
    match crate::db::max_nav_date(config::pool(), &code)
        .await
        .map_err(api_err)?
    {
        None => {
            let fund = crate::eastmoney::fetch_fund(&code).await.map_err(api_err)?;
            crate::db::upsert_fund(config::pool(), &fund)
                .await
                .map_err(api_err)?;
            Ok(Json(FundInfo {
                code,
                name: fund.name,
            }))
        }
        Some(last) => {
            let name = crate::db::fund_name(config::pool(), &code)
                .await
                .map_err(api_err)?
                .unwrap_or_else(|| "unknown".to_string());
            let today = Local::now().date_naive();
            let navs = crate::eastmoney::fetch_nav_range(&code, last + Duration::days(1), today)
                .await
                .map_err(api_err)?;
            if navs.is_empty() {
                Ok(Json(FundInfo { code, name }))
            } else {
                let fund = crate::eastmoney::Fund { code, name, navs };
                crate::db::upsert_fund(config::pool(), &fund)
                    .await
                    .map_err(api_err)?;
                Ok(Json(FundInfo {
                    code: fund.code,
                    name: fund.name,
                }))
            }
        }
    }
}

async fn fund_navs(Path(code): Path<String>) -> Result<Json<Vec<NavPoint>>, ApiError> {
    let navs = crate::db::load_navs(config::pool(), &code)
        .await
        .map_err(api_err)?;
    Ok(Json(
        navs.into_iter()
            .map(|n| NavPoint {
                date: n.date.to_string(),
                unit_nav: n.unit_nav,
                accum_nav: n.accum_nav,
                daily_return: n.daily_return,
            })
            .collect(),
    ))
}

async fn fund_range(Path(code): Path<String>) -> Result<Json<NavRange>, ApiError> {
    let navs = crate::db::load_navs(config::pool(), &code)
        .await
        .map_err(api_err)?;
    let (first, last) = match (navs.first(), navs.last()) {
        (Some(f), Some(l)) => (f.date, l.date),
        _ => {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                format!("no data stored for fund code {code}"),
            ));
        }
    };
    Ok(Json(NavRange {
        from: first.to_string(),
        to: last.to_string(),
    }))
}

async fn fund_rules(Path(code): Path<String>) -> Result<Json<Vec<FeeTier>>, ApiError> {
    let rule = crate::db::load_rules(config::pool(), &code)
        .await
        .map_err(api_err)?;
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
    Ok(Json(out))
}

async fn list_strategies() -> Json<Vec<StrategyInfo>> {
    Json(
        strategy::names()
            .iter()
            .map(|name| StrategyInfo {
                name: (*name).to_string(),
                description: match *name {
                    "buy_hold" => "invest once and hold".to_string(),
                    "dca" => "invest a fixed amount on a regular schedule".to_string(),
                    _ => String::new(),
                },
            })
            .collect(),
    )
}

async fn run_backtest(Json(input): Json<BacktestInput>) -> Result<Json<BacktestReport>, ApiError> {
    use chrono::NaiveDate;
    let navs = crate::db::load_navs(config::pool(), &input.code)
        .await
        .map_err(api_err)?;
    let from = input
        .from
        .as_deref()
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()
        .map_err(api_err)?;
    let to = input
        .to
        .as_deref()
        .map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d"))
        .transpose()
        .map_err(api_err)?;
    let navs: Vec<_> = navs
        .into_iter()
        .filter(|nav| from.is_none_or(|f| nav.date >= f) && to.is_none_or(|t| nav.date <= t))
        .collect();
    if navs.is_empty() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            format!("no data stored for fund code {}", input.code),
        ));
    }
    let start = navs.first().expect("non-empty").date;
    let end = navs.last().expect("non-empty").date;
    let days = navs.len();

    let stored = crate::db::load_rules(config::pool(), &input.code)
        .await
        .map_err(api_err)?;
    let fee_rule = stored.unwrap_or_else(|| FeeRule {
        subscribe: vec![],
        redeem: vec![],
    });
    let mut fee_rule = rules::Fifo::new(fee_rule.subscribe, fee_rule.redeem);
    let arg = StrategyArg::Bundled(input.strategy.clone());
    let mut strategy = strategy::load(&arg, input.initial, input.dca_amount, input.dca_interval)
        .map_err(api_err)?;
    let result = engine_simulate(&navs, &mut fee_rule, strategy.as_mut()).map_err(api_err)?;
    let report = report::build(start, end, days, &result);

    let curve = result
        .snapshots
        .iter()
        .map(|s| CurvePoint {
            date: s.date.to_string(),
            market_value: s.market_value(),
        })
        .collect();

    let nav_curve = result
        .snapshots
        .iter()
        .map(|s| CurvePoint {
            date: s.date.to_string(),
            market_value: s.unit_nav,
        })
        .collect();

    let markers = result
        .transactions
        .iter()
        .map(|t| BacktestMarker {
            date: t.date.to_string(),
            kind: match t.kind {
                crate::sim::event::TransactionKind::Invest { .. } => "buy".to_string(),
                crate::sim::event::TransactionKind::Redeem { .. } => "sell".to_string(),
            },
        })
        .collect();

    let mut peak = f64::NEG_INFINITY;
    let return_curve = result
        .snapshots
        .iter()
        .map(|s| CurvePoint {
            date: s.date.to_string(),
            market_value: s.return_pct(),
        })
        .collect();
    let drawdown_curve = result
        .snapshots
        .iter()
        .map(|s| {
            let value = s.market_value();
            peak = peak.max(value);
            let dd = if peak > 0.0 {
                (peak - value) / peak * 100.0
            } else {
                0.0
            };
            CurvePoint {
                date: s.date.to_string(),
                market_value: dd,
            }
        })
        .collect();
    let invested_curve = result
        .snapshots
        .iter()
        .map(|s| CurvePoint {
            date: s.date.to_string(),
            market_value: s.cumulative_investment,
        })
        .collect();
    let redeemed_curve = result
        .snapshots
        .iter()
        .map(|s| CurvePoint {
            date: s.date.to_string(),
            market_value: s.cumulative_redemption,
        })
        .collect();

    Ok(Json(BacktestReport {
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
        nav_curve,
        markers,
        return_curve,
        drawdown_curve,
        invested_curve,
        redeemed_curve,
    }))
}

use crate::rules::Rule;
use crate::sim::engine;
use crate::sim::engine::SimulationResult;

fn engine_simulate(
    navs: &[crate::eastmoney::Nav],
    fee_rule: &mut dyn Rule,
    strategy: &mut dyn strategy::Strategy,
) -> crate::error::Result<SimulationResult> {
    engine::simulate(navs, fee_rule, strategy)
}
