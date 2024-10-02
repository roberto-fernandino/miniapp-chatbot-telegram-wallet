use solana_program::native_token::lamports_to_sol;
use anyhow::Result;
use solana_sdk::account;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use solana_transaction_status::EncodedTransactionWithStatusMeta;



struct PumpSwap {
    sol_amount: u64,
    token_amount: u64,
}

pub fn is_pump_swap(transaction: &EncodedTransactionWithStatusMeta) -> Result<bool> {
    if transaction.meta.is_some() {
        let is_pump = transaction
            .clone()
            .meta
            .unwrap()
            .log_messages
            .unwrap()
            .iter()
            .any(|instruction| {
                instruction.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") == true
            });
        Ok(is_pump)
    } else {
        Ok(false)
    }
}

pub fn info_pump_swap(
    transaction: &EncodedTransactionWithStatusMeta,
    account_involved: &Pubkey,
) -> Result<(String, String)> {
    if transaction.meta.is_some() {
        let is_buy = transaction
            .clone()
            .meta
            .expect("No meta data in transaction")
            .log_messages
            .expect("No log messages in transaction")
            .iter()
            .any(|instruction| instruction.contains("Instruction: Buy") == true);

        let is_sell = transaction
            .clone()
            .meta
            .expect("No meta data in transaction")
            .log_messages
            .expect("No log messages in transaction")
            .iter()
            .any(|instruction| instruction.contains("Instruction: Sell") == true);

        let pre_token_balance = transaction
            .clone()
            .meta
            .expect("No meta data in transaction")
            .pre_token_balances
            .expect("No pre token balances in transaction");

        let post_token_balance = transaction
            .clone()
            .meta
            .expect("No meta data in transaction")
            .post_token_balances
            .expect("No post token balances in transaction");

        let pre_balances = transaction
            .clone()
            .meta
            .expect("No meta data in transaction")
            .pre_balances;

        let post_balances = transaction
            .clone()
            .meta
            .expect("No meta data in transaction")
            .post_balances;

        if is_sell {
            // Get the pre token amount
            let pre_token_amont = pre_token_balance
                .iter()
                .find(|balance| balance.owner.clone().expect("No owner") == account_involved.to_string())
                .expect("No token balance for account_involved")
                .ui_token_amount
                .ui_amount
                .expect("No pre token amount in pump sell transaction");

            // Get the post token amount
            let post_token_amount = post_token_balance
                .clone()
                .iter()
                .find(|balance| balance.owner.clone().expect("No owner") == account_involved.to_string())
                .expect("No token balance for account_involved")
                .ui_token_amount
                .ui_amount
                .unwrap_or_else(|| 0.0);

            let token_ca = post_token_balance
                .iter()
                .find(|balance| balance.owner.clone().expect("No owner") == account_involved.to_string())
                .expect("No token balance for account_involved")
                .mint
                .clone();

            let amount_token_sold = pre_token_amont - post_token_amount;
            let lamports_sol_change = post_balances[0] - pre_balances[0];
            println!(
                "Wallet: {} Sold {} Token({}) -> {} SOL",
                account_involved.to_string(),
                amount_token_sold,
                token_ca,
                lamports_to_sol(lamports_sol_change)
            );
            Ok((token_ca, "sell".to_string()))
        } else if is_buy {
            // Get the pre token amount
            let pre_token_amont = pre_token_balance
                .iter()
                .find(|balance| balance.owner.as_ref().expect("No owner") == &account_involved.to_string())
                .map(|balance| {
                    balance
                        .ui_token_amount
                        .ui_amount
                        .expect("No pre token amount in pump buy transaction")
                })
                .unwrap_or(0.0); // Get the post token amount

            let post_token_amount = post_token_balance
                .clone()
                .iter()
                .find(|balance| balance.owner.clone().expect("No owner") == account_involved.to_string())
                .expect("No token balance for account_involved")
                .ui_token_amount
                .ui_amount
                .unwrap_or_else(|| 0.0);

            let token_ca = post_token_balance
                .iter()
                .find(|balance| balance.owner.clone().expect("No owner") == account_involved.to_string())
                .expect("No token balance for account_involved")
                .mint
                .clone();

            let amount_token_bought = post_token_amount - pre_token_amont;
            let lamports_amount_change = pre_balances[0] - post_balances[0];
            println!(
                "Wallet: {} Bought {} SOL for -> {} Token({})",
                account_involved.to_string(),
                lamports_to_sol(lamports_amount_change),
                amount_token_bought,
                token_ca,
            );
            Ok((token_ca, "buy".to_string()))
        } else {
            Err(anyhow::anyhow!("Transaction is not a pump swap"))
        }
    } else {
        Err(anyhow::anyhow!("Transaction doesnt have meta data"))
    }
}



