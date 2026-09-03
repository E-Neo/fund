//! Fund backtesting domain and HTTP API.
//!
//! This crate contains the domain logic (data fetching, fee rules,
//! simulation, reports) plus an axum [`Router`] exposing the REST API under
//! `/api`.

pub mod api;
pub mod config;
pub mod db;
pub mod eastmoney;
pub mod error;
pub mod fees;
pub mod report;
pub mod rules;
pub mod sim;

pub use fund_types;
