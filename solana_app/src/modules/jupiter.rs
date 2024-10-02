use std::error::Error;
use anyhow::Result;
use solana_sdk::{
    native_token::lamports_to_sol, pubkey::Pubkey
};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransactionWithStatusMeta,
};

pub fn is_jupiter_swap(
    transaction: &EncodedTransactionWithStatusMeta,
) -> Result<bool> {
    let mut is_jup: bool = false;

    if transaction
        .clone()
        .meta
        .unwrap()
        .log_messages
        .unwrap()
        .len()
        > 9
    {
        // check if the transaction is a swap
        if let Some(meta) = transaction.clone().meta {
            let log_messages = meta.log_messages.unwrap();
            if log_messages.iter().any(|log_message| {
                log_message.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4")
            }) {
                // Transaction is a swap
                is_jup = true;
            }
        }
    }
    Ok(is_jup)
}

pub fn info_jupiter_swap(
    transaction: &EncodedTransactionWithStatusMeta,
    addres_involved: &Pubkey,
) -> Result<(String, String)> {
    let pre_balances = transaction.clone().meta.unwrap().pre_balances;
    let post_balances = transaction.clone().meta.unwrap().post_balances;



    if pre_balances[0] > post_balances[0] {

        // Get the token CA
        let token_ca = match transaction
            .meta
            .as_ref()
            .unwrap()
            .pre_token_balances
            .as_ref()
            .unwrap()
            .iter()
            .find(|token| token.mint != "So11111111111111111111111111111111111111112" && token.owner.as_ref().map(|s| s.as_str()) == Some(addres_involved.to_string().as_str()))
            .map(|token| token.mint.clone()) {
                Some(mint) => mint,
                None => match transaction
                    .meta
                    .as_ref()
                    .unwrap()
                    .pre_token_balances
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|token| token.mint != "So11111111111111111111111111111111111111112")
                    .map(|token| token.mint.clone()) {
                        Some(mint) => mint,
                        None => {println!("Couldn't get token_ca from transaction."); "".to_string()}
                    }
            };

        let lamports_buy_amount = pre_balances[0] - post_balances[0];
        let sol_buy_amount = lamports_to_sol(lamports_buy_amount);

        let amount_token_bought = transaction
            .clone()
            .meta
            .unwrap()
            .post_token_balances
            .expect("Couldn't get post_token_balances from transaction.")
            .iter()
            .find(|token_balance| {
                token_balance.mint == token_ca
                    && token_balance.owner.as_ref().map(|s| s.as_str())
                        == Some(addres_involved.to_string().as_str())
            })
            .expect("Couldn't find the token with the specified mint and owner.")
            .ui_token_amount
            .ui_amount
            .unwrap();
        println!(
            "Wallet: {} bought {} SOL -> {} Token({})",
            addres_involved.to_string(),
            sol_buy_amount, amount_token_bought, token_ca
        );
        return Ok((token_ca, "buy".to_string()));
    } else {
        let token_ca = match transaction
            .meta
            .as_ref()
            .unwrap()
            .pre_token_balances
            .as_ref()
            .unwrap()
            .iter()
            .find(|token| token.mint != "So11111111111111111111111111111111111111112" && token.owner.as_ref().map(|s| s.as_str()) == Some(addres_involved.to_string().as_str()))
            .map(|token| token.mint.clone()) {
                Some(mint) => mint,
                None => match transaction
                    .meta
                    .as_ref()
                    .unwrap()
                    .pre_token_balances
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|token| token.mint != "So11111111111111111111111111111111111111112")
                    .map(|token| token.mint.clone()) {
                        Some(mint) => mint,
                        None => {println!("Couldn't get token_ca from transaction."); "".to_string()}
                    }
            };

        let before_sell_token = transaction
            .clone()
            .meta
            .unwrap() // Unwrap the meta field
            .pre_token_balances // Access pre_token_balances
            .expect("(Couldn't get pre_token_balances from transaction.") // Handle not found case
            .iter()
            .find(|token_balance| {
                // Find the token balance
                token_balance.mint == token_ca // Check if mint matches
            && token_balance.owner.as_ref().map(|s| s.as_str()) // Check if owner matches
                == Some(addres_involved.to_string().as_str())
            })
            .expect("Couldn't find the token with the specified mint and owner.") // Handle not found case
            .ui_token_amount // Access ui_token_amount
            .ui_amount // Access ui_amount
            .unwrap(); // Unwrap the ui_amount

        let after_sell_token = transaction
            .clone()
            .meta
            .unwrap() // Unwrap the meta field
            .post_token_balances // Access post_token_balances
            .expect("(Couldn't get post_token_balances from transaction.") // Handle not found case
            .iter()
            .find(|token_balance| {
                // Find the token balance
                token_balance.mint == token_ca // Check if mint matches
            && token_balance.owner.as_ref().map(|s| s.as_str()) // Check if owner matches
                == Some(addres_involved.to_string().as_str())
            })
            .expect("Couldn't find the token with the specified mint and owner.") // Handle not found case
            .ui_token_amount // Access ui_token_amount
            .ui_amount // Access ui_amount
            .unwrap_or_else(|| 0.0); // Unwrap the ui_amount
        let lamports_sell_amount = post_balances[0] - pre_balances[0];
        let sol_sell_amount = lamports_to_sol(lamports_sell_amount);
        let sell_token_amount = before_sell_token - after_sell_token;
        println!(
            "Wallet: {} sold {} Token({}) -> {} SOL",
            addres_involved.to_string(),
            sell_token_amount, token_ca, sol_sell_amount
        );
        return Ok((token_ca, "sell".to_string()));
    }
}

pub fn check_jupiter_swap(
    transaction: &EncodedConfirmedTransactionWithStatusMeta,
)-> Result<bool> {
    let is_jup_swap: bool = match is_jupiter_swap(&transaction.transaction.clone()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Jupiter Swap Handler: {}", e);
            false
        }
    };
    
    Ok(is_jup_swap)
}
