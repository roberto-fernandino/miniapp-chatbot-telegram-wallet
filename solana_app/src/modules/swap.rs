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

    // Remove surrounding quotes from keys if present
    let api_public_key = user.api_public_key.trim_matches('"');
    let api_private_key = user.api_private_key.trim_matches('"');
    let organization_id = user.organization_id.trim_matches('"');
    let public_key = user.public_key.trim_matches('"');
    println!("@sign_and_send_swap_transaction/ api_public_key: {}", api_public_key);
    println!("@sign_and_send_swap_transaction/ api_private_key: {}", api_private_key);
    println!("@sign_and_send_swap_transaction/ organization_id: {}", organization_id);
    println!("@sign_and_send_swap_transaction/ public_key: {}", public_key);


    let turnkey_client = Turnkey::new_for_user(api_public_key, api_private_key, organization_id, public_key)?;
    println!("@sign_and_send_swap_transaction/ turnkey_client created: {:?}", turnkey_client);
    let pubkey = Pubkey::from_str(public_key).expect("Invalid pubkey");

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

    let mut transaction = match bincode::deserialize::<Transaction>(&transaction_data) {
        Ok(tx) => Some(tx),
        Err(e) => {
            println!("Failed to deserialize transaction: {:?}", e);
            println!("Transaction data (base64): {}", transaction.swap_transaction);
            println!("Transaction data (decoded): {:?}", transaction_data);
            None
        }
    };
    match rpc_client.get_latest_blockhash() {
        Ok(recent_blockhash) => {
            transaction.as_mut().unwrap().message.recent_blockhash = recent_blockhash;
            println!("@sign_and_send_swap_transaction/ recent_blockhash updated: {}", recent_blockhash);
        },
        Err(e) => {
            println!("Failed to fetch recent blockhash: {:?}", e);
            return Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to fetch recent blockhash: {:?}", e))));
        }
    }

    if let Some(mut transaction) = transaction {
        println!("@sign_and_send_swap_transaction/ transaction deserialized successfully");

        let key_info = KeyInfo {
           private_key_id: public_key.to_string(), // Ensure this is a String
           public_key: pubkey // Pubkey OBJ
        };
        println!("@sign_and_send_swap_transaction/ key_info created: {:?}", key_info);

        // Sign transaction
        println!("@sign_and_send_swap_transaction/ signing transaction");
        match turnkey_client.sign_transaction(&mut transaction, key_info).await {
            Ok((tx, _sig)) => {
                println!("@sign_and_send_swap_transaction/ transaction signed");

                println!("@sign_and_send_swap_transaction/ sending transaction");
                match rpc_client.send_and_confirm_transaction(&tx) {
                    Ok(tx_sig) => {
                        println!("@sign_and_send_swap_transaction/ transaction confirmed: {:?}", tx_sig);
                        Ok(tx_sig)
                    },
                     Err(e) => {
                        println!("@sign_and_send_swap_transaction/ detailed error: {:?}", e);
            
                        // Get current blockhash
                        let recent_blockhash = rpc_client.get_latest_blockhash().map_err(|e| TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to get latest blockhash: {:?}", e)))).unwrap();
                        println!("@sign_and_send_swap_transaction/ recent blockhash: {:?}", recent_blockhash);
            
                        // Get account balance
                        let balance = rpc_client.get_balance(&pubkey).map_err(|e| TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to get account balance: {:?}", e)))).unwrap();
                        println!("@sign_and_send_swap_transaction/ account balance: {} SOL", balance as f64 / 1e9);
            
                        // Get transaction fee
                        let fee = rpc_client.get_fee_for_message(&transaction.message).map_err(|e| TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to get transaction fee: {:?}", e)))).unwrap();
                        println!("@sign_and_send_swap_transaction/ transaction fee: {} SOL", fee as f64 / 1e9);
                        Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to send transaction: {:?}", e))))
                    }
                }
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
