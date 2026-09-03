mod ui;

use axum::Router;
use clap::Parser;
use fund_api::config::{init, load_config};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    use std::net::SocketAddr;

    let args = Args::parse();
    let home = PathBuf::from(&args.home);
    let config = load_config(&home).expect("failed to load config");
    init(&home).await.expect("failed to open database");

    let addr: SocketAddr = config.server.addr.parse().expect("invalid server addr");

    let app: Router = Router::new()
        .merge(fund_api::api::router())
        .merge(ui::router());

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("home: {}", home.display());
    println!("listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

#[derive(clap::Parser)]
#[command(name = "fund", about = "Fund backtesting web server")]
struct Args {
    /// Home directory containing config.toml and db.sqlite3.
    #[arg(long, default_value = ".")]
    home: String,
}
