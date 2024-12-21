use anyhow::Result;
use serde::Serialize;
use serde_json::Value as JsonValue;
use solana_account_decoder::UiAccountData;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{native_token::lamports_to_sol, pubkey::Pubkey};
use std::sync::Arc;

/// Struct representing a token balance
#[derive(Debug, Serialize)]
pub struct TokenBalance {
    pub sol_amount: f64,
    pub lamports_amount: u64,
    pub mint: String,
    pub token_ui_amount: f64,
    pub token_amount: u64,
}

/// Struct representing multiple token balances
#[derive(Debug, serde::Serialize)]
pub struct TokensBalance {
    pub token_balance: Vec<TokenBalance>,
}

impl TokensBalance {
    /// Add a new TokenBalance to the list
    pub fn add_token_balance(&mut self, token_balance: TokenBalance) {
        self.token_balance.push(token_balance);
    }
}

/// Get token balances for a wallet
///
/// # Parameters
/// - `client`: Arc<RpcClient> - Thread-safe reference to the RPC client
/// - `wallet_pubkey`: &Pubkey - Reference to the wallet's public key
///
/// # Returns
/// - `Result<TokensBalance>`: The token balances or an error
pub fn get_tokens_balance(client: Arc<RpcClient>, wallet_pubkey: &Pubkey) -> Result<TokensBalance> {
    // Fetch token accounts for the wallet
    let token_accounts = client.get_token_accounts_by_owner(
        wallet_pubkey,
        solana_client::rpc_request::TokenAccountsFilter::ProgramId(spl_token::id()),
    )?;

    let mut tokens_balance = TokensBalance {
        token_balance: Vec::new(),
    };

    // Process each token account
    for account in token_accounts {
        let lamports_amount = account.account.lamports;
        let sol_amount = lamports_to_sol(lamports_amount);

        // Handle UiAccountData::Json
        if let UiAccountData::Json(parsed_account) = &account.account.data {
            let parsed_data: &JsonValue = &parsed_account.parsed;

            if let Some(token_ui_amount) = parsed_data["info"]["tokenAmount"]["uiAmount"].as_f64() {
                if let Some(token_mint) = parsed_data["info"]["mint"].as_str() {
                    let token_amount = parsed_data["info"]["tokenAmount"]["amount"]
                        .as_str()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap();

                    let token_balance = TokenBalance {
                        sol_amount,
                        lamports_amount,
                        token_amount,
                        mint: token_mint.to_string(),
                        token_ui_amount,
                    };

                    tokens_balance.token_balance.push(token_balance);
                }
            }
        }
    }

    Ok(tokens_balance)
}
