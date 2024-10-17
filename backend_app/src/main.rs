use anyhow::Result;
use crate::routes::{index, set_copy_trade_wallet, get_copy_trades, delete_copy_trade_wallet, set_user_session};
mod routes;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = tide::new();
    println!("Listening on port 8000");

    app.at("/").get(index);
    app.at("/set_copy_trade_wallet").post(set_copy_trade_wallet);
    app.at("/delete_copy_trade_wallet/:user_id/:copy_trade_address").delete(delete_copy_trade_wallet);
    app.at("/get_copy_trades/:user_id").get(get_copy_trades);
    app.at("/set_user_session").post(set_user_session);
    app.listen("0.0.0.0:8000").await?;

    Ok(())
}

