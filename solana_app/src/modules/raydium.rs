use anyhow::Result;
use solana_sdk::{native_token::lamports_to_sol, pubkey::Pubkey};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransactionWithStatusMeta,
};
use std::error::Error;

pub fn info_raydium_swap(transaction: &EncodedTransactionWithStatusMeta, addres_involved: &Pubkey) -> Result<(String, String)> {
    // Fee
    let pre_balances = transaction.clone().meta.unwrap().pre_balances;
    let post_balances = transaction.clone().meta.unwrap().post_balances;

    let token_ca = transaction
        .clone()
        .meta
        .unwrap()
        .post_token_balances
        .unwrap()
        .iter()
        .find(|token| token.mint != "So11111111111111111111111111111111111111112")
        .map(|token| token.mint.clone())
        .unwrap();

    // Avoid getting wrong transactions
    if pre_balances[0] > post_balances[0] {
        let sol_amount = lamports_to_sol(
            (transaction.clone().meta.unwrap().pre_balances[0])
                - (transaction.clone().meta.unwrap().post_balances[0]),
        );
        let amount_token_bought = transaction
            .clone()
            .meta
            .unwrap()
            .post_token_balances
            .unwrap()[0]
            .ui_token_amount
            .ui_amount
            .unwrap_or_else(|| 0.0);
        println!(
            "Wallet: {} bought {} SOL for -> {} Token({})",
            addres_involved,
            sol_amount,
            amount_token_bought,
            token_ca,
        );
        return Ok((token_ca, "buy".to_string()));
    } else {
        let sell_amount = (transaction
            .clone()
            .meta
            .unwrap()
            .pre_token_balances
            .unwrap()[transaction
            .clone()
            .meta
            .unwrap()
            .pre_token_balances
            .unwrap()
            .len()
            - 1]
        .ui_token_amount
        .ui_amount
        .expect("Couldn't get pre_token_balances last ui_amount from last transaction."))
            - (transaction
                .clone()
                .meta
                .unwrap()
                .post_token_balances
                .unwrap()[transaction
                .clone()
                .meta
                .unwrap()
                .post_token_balances
                .unwrap()
                .len()
                - 1]
            .ui_token_amount
            .ui_amount
            .unwrap_or_else(|| 0.0));
        let sol_amount = lamports_to_sol(
            transaction.clone().meta.unwrap().post_balances[0]
                - transaction.clone().meta.unwrap().pre_balances[0],
        );
        println!(
            "Wallet: {} sold {} Token({}) -> {} SOL",
            addres_involved, sell_amount, token_ca, sol_amount
        );
        return Ok((token_ca, "sell".to_string()));
    }
}

pub fn is_raydium_swap(
    transaction: &EncodedTransactionWithStatusMeta,
) -> Result<bool> {
    let mut is_swap: bool = false;

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
                log_message.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")
            }) {
                // Transaction is a swap
                is_swap = true;
            }
        }
    }
    Ok(is_swap)
}



pub fn check_raydium_swap(
    transaction: &EncodedConfirmedTransactionWithStatusMeta,
)-> Result<bool> {
    let is_raydium_swap = match is_raydium_swap(&transaction.transaction.clone()) {
        Ok(r) => r,
        Err(e) => {
            println!("is raydium swap: {}", e);
            false
        }
    };
    
    Ok(is_raydium_swap)
}
