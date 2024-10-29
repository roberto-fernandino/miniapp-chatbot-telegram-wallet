use solana_client::{rpc_client::RpcClient, rpc_config::RpcTransactionConfig};
use solana_sdk::transaction::Transaction;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, TransactionBinaryEncoding,
    UiMessage, UiTransactionEncoding,
};
use std::error::Error;
use std::fs::File;
use std::str::FromStr;
use anyhow::Result;

/// Decode a signature and get the transaction
/// 
/// # Arguments
/// 
/// * `signature_str` - The signature
/// * `rpc_client` - The RPC client
/// 
/// # Returns
/// 
/// The transaction
pub fn decode_signature_get_transaction(
    signature_str: &str,
    rpc_client: &RpcClient,
) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
    let decoded_bytes = match bs58::decode(signature_str).into_vec() {
        Ok(bytes) => {
            if bytes.len() != 64 {
                return Err(anyhow::anyhow!("Decoded bytes length is not 64"));
            }
            bytes
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to decode base58 string: {}", e));
        }
    };

    let mut signature_bytes = [0u8; 64];
    signature_bytes.copy_from_slice(&decoded_bytes);
    let signature = Signature::from(signature_bytes);

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    match rpc_client.get_transaction_with_config(&signature, config) {
        Ok(transaction_with_meta) => {
            Ok(transaction_with_meta)
        }
        Err(e) => Err(anyhow::anyhow!("Failed to fetch transaction: {}", e)),
    }
}

/// Get the account involved in a transaction
/// 
/// # Arguments
/// 
/// * `transaction` - The transaction
/// 
/// # Returns
/// 
/// The account involved in the transaction
pub fn get_account_involved_in_transaction(
    transaction: &EncodedConfirmedTransactionWithStatusMeta,
) -> Result<Pubkey, Box<dyn Error + Send + Sync>> {
    // Create account involved
    let mut account_involved: Option<Pubkey> = None;
    //
    match &transaction.transaction.transaction {
        EncodedTransaction::Json(ref ui_transaction) => {
            if let UiMessage::Raw(ref ui_message) = &ui_transaction.message {
                account_involved = Some(
                    Pubkey::from_str(ui_message.account_keys[0].clone().as_str())
                        .expect("Failed to parse account_involved to Pubkey"),
                );
            } else {
                eprintln!("Expected Raw variant of UiMessage");
            }
        }
        EncodedTransaction::Binary(encoded_transaction, TransactionBinaryEncoding::Base58) => {
            let decoded_bytes: Vec<u8> = bs58::decode(encoded_transaction).into_vec()?;
            let tx: Transaction = bincode::deserialize(&decoded_bytes)?;
            account_involved = Some(tx.message.account_keys[0]);
        }
        _ => {}
    }
    Ok(account_involved.expect("Account involved not found"))
}