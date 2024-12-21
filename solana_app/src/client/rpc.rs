use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{native_token::lamports_to_sol, pubkey::Pubkey};
use std::{env, str::FromStr, sync::Arc};

/// Initialize the RPC client
///
/// # Returns
/// - `Result<RpcClient>`: The initialized RPC client or an error
pub async fn init_rpc_client() -> Result<RpcClient> {
    // Initialize logger
    let http_url = env::var("NODE_HTTP").expect("NODE_HTTP must be set");
    pretty_env_logger::formatted_timed_builder()
        .filter(None, log::LevelFilter::Info)
        .init();

    // Create and return the RPC client
    Ok(RpcClient::new(http_url))
}

pub fn get_sol_balance(address: &str) -> Result<f64> {
    let client = Arc::new(RpcClient::new(
        env::var("NODE_HTTP").expect("NODE_HTTP must be set"),
    ));
    let balance = client.get_balance(&Pubkey::from_str(address).unwrap())?;
    Ok(lamports_to_sol(balance))
}
