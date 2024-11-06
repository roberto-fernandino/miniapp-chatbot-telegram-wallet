use crate::turnkey::client::Turnkey;
use std::env;
use crate::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use crate::turnkey::client::KeyInfo;
use crate::turnkey::errors::TurnkeyResult;
use crate::modules::swap::User;
use solana_sdk::{
    native_token::lamports_to_sol,
    transaction::Transaction,
    signature::Signature,
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



/// Sign and send a transaction
/// 
/// # Arguments
/// 
/// * `transaction` - The transaction to send
/// * `user` - The user to sign the transaction
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
pub async fn sign_and_send_transaction(mut transaction: Transaction, user: User) -> TurnkeyResult<Signature> {
    let turnkey_client = Turnkey::new_for_user(&user.api_public_key, &user.api_private_key, &user.organization_id, &user.public_key)?;
    println!("@sign_and_send_transaction/ turnkey_client: {:?}", turnkey_client);

    let pubkey = Pubkey::from_str(&user.public_key).unwrap();
    let key_info = KeyInfo {
        private_key_id: user.public_key.to_string(),
        public_key: pubkey
    };
    println!("@sign_and_send_transaction/ signing transaction");
    let tx_and_sig = turnkey_client.sign_transaction(&mut transaction, key_info).await?;
    println!("@sign_and_send_transaction/ tx_and_sig: {:?}", tx_and_sig);
    let rpc_client = RpcClient::new(env::var("NODE_HTTP").expect("NODE_HTTP must be set"));
    println!("@sign_and_send_transaction/ sending and confirming transaction");
    let signature = rpc_client.send_and_confirm_transaction(&tx_and_sig.0).expect("Failed to send and confirm transaction");
    println!("@sign_and_send_transaction/ signature: {:?}", signature);
    Ok(signature)
}