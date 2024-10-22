use base64::Engine;
use crate::turnkey::errors::TurnkeyError;
use {
    super::matis::SwapTransaction, crate::turnkey::{
        client::{KeyInfo, Turnkey}, errors::TurnkeyResult
    }, bincode::deserialize, serde::{Deserialize, Serialize}, solana_client::rpc_client::RpcClient, solana_sdk::{
        pubkey::Pubkey, signature::Signature, transaction::Transaction, transaction::VersionedTransaction
    }, std::{env, str::FromStr}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub api_public_key: String,
    pub api_private_key: String,
    pub organization_id: String,
    pub public_key: String,
}

pub async fn sign_and_send_swap_transaction(transaction: SwapTransaction, user: User) -> TurnkeyResult<Signature> {
    // Initialize Turnkey client
    println!("@sign_and_send_swap_transaction/ user: {:?}", user);
    println!("@sign_and_send_swap_transaction/ transaction: {:?}", transaction);
    println!("@sign_and_send_swap_transaction/ api_public_key: {}", user.api_public_key);
    println!("@sign_and_send_swap_transaction/ api_private_key: {}", user.api_private_key);
    println!("@sign_and_send_swap_transaction/ organization_id: {}", user.organization_id);
    println!("@sign_and_send_swap_transaction/ public_key: {}", user.public_key);

    let turnkey_client = Turnkey::new_for_user(&user.api_public_key, &user.api_private_key, &user.organization_id, &user.public_key)?;
    println!("@sign_and_send_swap_transaction/ turnkey_client created: {:?}", turnkey_client);
    let pubkey = Pubkey::from_str(&user.public_key).expect("Invalid pubkey");

    // Initialize RPC client
    let rpc_client = RpcClient::new(env::var("NODE_HTTP").expect("NODE_HTTP must be set"));
    println!("@sign_and_send_swap_transaction/ connected to RPC client");

    // Decode transaction
    println!("@sign_and_send_swap_transaction/ decoding transaction");
    let engine = base64::engine::general_purpose::STANDARD;

    let transaction_data = engine.decode(&transaction.swap_transaction).map_err(|e| {
        println!("Base64 decoding error: {:?}", e);
        e
    }).expect("Failed to decode transaction");
    println!("@sign_and_send_swap_transaction/ transaction decoded, length: {}", transaction_data.len());

    let transaction = match bincode::deserialize::<Transaction>(&transaction_data) {
        Ok(tx) => Some(tx),
        Err(e) => {
            println!("Failed to deserialize transaction: {:?}", e);
            println!("Transaction data (base64): {}", transaction.swap_transaction);
            println!("Transaction data (decoded): {:?}", transaction_data);
            None
        }
    };

    if let Some(mut transaction) = transaction {
        println!("@sign_and_send_swap_transaction/ transaction deserialized successfully");

        // Get latest blockhash
        let key_info = KeyInfo {
           private_key_id: user.public_key,
           public_key: pubkey
        };
        println!("@sign_and_send_swap_transaction/ key_info created: {:?}", key_info);

        // Sign transaction
        println!("@sign_and_send_swap_transaction/ signing transaction");
        match turnkey_client.sign_transaction(&mut transaction, key_info).await {
            Ok((tx, _sig)) => {
                println!("@sign_and_send_swap_transaction/ transaction signed");

                println!("@sign_and_send_swap_transaction/ sending transaction");
                let tx_sig = rpc_client.send_and_confirm_transaction(&tx).expect("Failed to send transaction");
                println!("@sign_and_send_swap_transaction/ transaction confirmed: {:?}", tx_sig);

                Ok(tx_sig)
            }
            Err(e) => {
                println!("Failed to sign transaction: {:?}", e);
                Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to sign transaction: {:?}", e))))
            }
        }
    } else {
        Err(TurnkeyError::from(Box::<dyn std::error::Error>::from("Failed to deserialize transaction".to_string())))
    }
}
