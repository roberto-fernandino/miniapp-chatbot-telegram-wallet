use solana_sdk::{
    native_token::{lamports_to_sol},
    transaction::Transaction,
};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiMessage,
};
use anyhow::Result;

#[derive(Debug)]
pub struct Transfer {
    pub from: String,
    pub to: String,
    pub lamports_amount: u64,
    pub sol_amount: f64,
}
impl Transfer {
    pub fn none() -> Self {
        Transfer {
            from: "".to_string(),
            to: "".to_string(),
            lamports_amount: 0,
            sol_amount: 0.0,
        }
    }
}



pub fn handle_transfer_transaction(
    encoded_confirmed_transaction_with_status_meta: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<Transfer> {
    match encoded_confirmed_transaction_with_status_meta
        .transaction
        .transaction
    {
        EncodedTransaction::Binary(
            ref encoded_transaction,
            solana_transaction_status::TransactionBinaryEncoding::Base58,
        ) => {
            let pre_balances = encoded_confirmed_transaction_with_status_meta
                .transaction
                .clone()
                .meta
                .expect("Theres no meta in tx")
                .pre_balances;

            let post_balances = encoded_confirmed_transaction_with_status_meta
                .transaction
                .clone()
                .meta
                .expect("Theres no meta in tx")
                .post_balances;

            let lamports_amount_sent = pre_balances[0] - post_balances[0];
            let sol_amount_sent = lamports_to_sol(lamports_amount_sent);
            let decoded_bytes = bs58::decode(encoded_transaction).into_vec()?;
            let tx: Transaction = bincode::deserialize(&decoded_bytes)?;
            Ok(Transfer {
                from: tx.message.account_keys[0].to_string(),
                to: tx.message.account_keys[1].to_string(),
                lamports_amount: lamports_amount_sent,
                sol_amount: sol_amount_sent,
            })
        }

        EncodedTransaction::LegacyBinary(ref encoded_transaction) => {
            let pre_balances = encoded_confirmed_transaction_with_status_meta
                .transaction
                .clone()
                .meta
                .expect("Theres no meta in tx")
                .pre_balances;

            let post_balances = encoded_confirmed_transaction_with_status_meta
                .transaction
                .clone()
                .meta
                .expect("Theres no meta in tx")
                .post_balances;

            let lamports_amount_sent = pre_balances[0] - post_balances[0];
            let sol_amount_sent = lamports_to_sol(lamports_amount_sent);
            let decoded_bytes = bs58::decode(encoded_transaction).into_vec()?;
            let tx: Transaction = bincode::deserialize(&decoded_bytes)?;
            Ok(Transfer {
                from: tx.message.account_keys[0].to_string(),
                to: tx.message.account_keys[1].to_string(),
                lamports_amount: lamports_amount_sent,
                sol_amount: sol_amount_sent,
            })
        }
        EncodedTransaction::Json(ref ui_transaction) => {
            let pre_balances = encoded_confirmed_transaction_with_status_meta
                .transaction
                .clone()
                .meta
                .expect("Theres no meta in tx")
                .pre_balances;

            let post_balances = encoded_confirmed_transaction_with_status_meta
                .transaction
                .clone()
                .meta
                .expect("Theres no meta in tx")
                .post_balances;
            let lamports_amount = pre_balances[0] - post_balances[0];
            match ui_transaction.message {
                UiMessage::Raw(ref raw_message) => Ok(Transfer {
                    from: raw_message.account_keys[0].to_string(),
                    to: raw_message.account_keys[1].to_string(),
                    lamports_amount: lamports_amount,
                    sol_amount: lamports_to_sol(lamports_amount),
                }),
                _ => Err(anyhow::anyhow!("Unsuported message type in Json transfer")),
            }
        }
        _ => Err(anyhow::anyhow!("Unsuported transaction encoding")),
    }
}


