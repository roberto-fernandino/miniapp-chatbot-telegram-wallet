use anyhow::Result;
use crate::routes::{index, add_or_update_user, get_all_users, get_user, add_wallet_to_user, get_user_wallets, set_copy_trade_wallet, get_copy_trades};
mod routes;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = tide::new();
    println!("Listening on port 8000");

    app.at("/").get(index);
    app.at("/add_or_update_user").post(add_or_update_user);
    app.at("/all_users").get(get_all_users);
    app.at("/user/:user_id").get(get_user);
    app.at("/add_wallet_to_user").post(add_wallet_to_user);
    app.at("/user_wallets/:user_id").get(get_user_wallets);
    app.at("/set_copy_trade_wallet").post(set_copy_trade_wallet);
    app.at("/get_copy_trades/:user_id").get(get_copy_trades);
    app.listen("0.0.0.0:8000").await?;

    Ok(())
}

