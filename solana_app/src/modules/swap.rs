use base64::Engine;
use solana_sdk::signature::Signature;
use tokio::time::{sleep, Duration};
use anyhow::anyhow;
use serde_json::json;
use jito_sdk_rust::JitoJsonRpcSDK;
use anyhow::Result;
use solana_sdk::system_instruction;
use solana_sdk::commitment_config::CommitmentConfig;
use crate::turnkey::errors::TurnkeyError;
use {
    super::matis::SwapTransaction, crate::turnkey::{
        client::{KeyInfo, Turnkey}, errors::TurnkeyResult
    }, serde::{Deserialize, Serialize}, solana_client::rpc_client::RpcClient, solana_sdk::{
        pubkey::Pubkey, transaction::Transaction
    }, std::{env, str::FromStr}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub api_public_key: String,
    pub api_private_key: String,
    pub organization_id: String,
    pub public_key: String,
}

#[derive(Debug)]
struct BundleStatus {
    confirmation_status: Option<String>,
    err: Option<serde_json::Value>,
    transactions: Option<Vec<String>>,
}

/// Sign and send a swap transaction
/// 
/// # Description
/// 
/// This function signs a the swap and jito TX using Turnkey and sends the swap to the RPC node.
/// It then creates a bundle for the two transactions and sends it to the JitoAPI.
/// It then checks the status of the bundle until it is finalized.
/// 
/// # Arguments
/// 
/// * `transaction` - The swap transaction to send
/// * `user` - The user to sign the transaction
/// * `jito_tip_amount` - The amount of Jito tokens to send as a tip
/// 
/// # Returns
/// 
/// A result indicating the success of the operation
pub async fn sign_and_send_swap_transaction(transaction: SwapTransaction, user: User, jito_tip_amount: u64) -> TurnkeyResult<String> {
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
    let key_info = KeyInfo {
        private_key_id: public_key.to_string(),
        public_key: pubkey
    };
    let jito_sdk = JitoJsonRpcSDK::new(env::var("JITO_BLOCK_ENGINE_URL").expect("JITO_BLOCK_ENGINE_URL must be set").as_str(), None);
    let jito_tip_ix = system_instruction::transfer(&pubkey, &Pubkey::from_str(&jito_sdk.get_random_tip_account().await.unwrap()).unwrap(), jito_tip_amount);
    let mut transaction = Transaction::new_with_payer(&[jito_tip_ix], Some(&pubkey));
    let jito_serialized_tx = match turnkey_client.sign_transaction(&mut transaction, key_info.clone()).await {
        Ok((signed_tx, _sig)) => {
            engine.encode(bincode::serialize(&signed_tx).unwrap())
        }
        Err(e) => {
            return Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(
                format!("Failed to sign transaction: {:?}", e)
            )))
        }
    };
    let (swap_tx, swap_sig) = match bincode::deserialize::<Transaction>(&transaction_data) {
        Ok(mut tx) => {
            // Sign transaction once
            match turnkey_client.sign_transaction(&mut tx, key_info).await {
                Ok((signed_tx, sig)) => {
                    rpc_client.send_transaction(&signed_tx).expect("Failed to send transaction to RPC.");
                    Ok::<(String, Signature), TurnkeyError>((engine.encode(bincode::serialize(&signed_tx).unwrap()), sig))
                }
                Err(e) => {
                    return Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(
                        format!("Failed to sign transaction: {:?}", e)
                    )))
                } 
            }
        },
        Err(e) => return Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(
            format!("Failed to deserialize transaction: {:?}", e)
        )))
    }?;
    let bundle = json!([
        jito_serialized_tx,
        swap_tx
    ]);
    let uuid = None;
    let response = jito_sdk.send_bundle(Some(bundle), uuid).await.expect("Failed to send bundle");
    let bundle_uuid = response["result"].as_str().ok_or_else(|| anyhow!("Failed to get bundle UUID")).expect("Failed to get bundle UUID");
    println!("@sign_and_send_swap_transaction/ sent bundle, bundle_uuid: {}", bundle_uuid);
    let max_retries = 10;
    let retry_delay = Duration::from_secs(2);

    for attempt in 1..=max_retries {
        println!("@sign_and_send_swap_transaction/ checking final bundle status (attempt {}/{})", attempt, max_retries);
        let status_response = jito_sdk.get_in_flight_bundle_statuses(vec![bundle_uuid.to_string()]).await.expect("Failed to get bundle status");
        let is_tx_confirmed = rpc_client.confirm_transaction_with_commitment(&swap_sig, CommitmentConfig::confirmed()).expect("Failed to confirm transaction").value;
        if is_tx_confirmed {
            println!("@sign_and_send_swap_transaction/ transaction confirmed");
            break;
        }
        if let Some(result) = status_response.get("result") {
             if let Some(value) = result.get("value") {
                 if let Some(statuses) = value.as_array() {
                     if let Some(bundle_status) = statuses.get(0) {
                         if let Some(status) = bundle_status.get("status") {
                             match status.as_str() {
                                 Some("Landed") => {
                                     println!("Bundle landed on-chain. Checking final status...");
                                     check_final_bundle_status(&jito_sdk, bundle_uuid).await.expect("Failed to check final bundle status");
                                 },
                                 Some("Pending") => {
                                     println!("Bundle is pending. Waiting...");
                                 },
                                 Some(status) => {
                                     println!("Unexpected bundle status: {}. Waiting...", status);
                                 },
                                 None => {
                                     println!("Unable to parse bundle status. Waiting...");
                                 }
                             }
                         } else {
                             println!("Status field not found in bundle status. Waiting...");
                         }
                     } else {
                         println!("Bundle status not found. Waiting...");
                     }
                 } else {
                     println!("Unexpected value format. Waiting...");
                 }
             } else {
                 println!("Value field not found in result. Waiting...");

             }
         } else if let Some(error) = status_response.get("error") {
             println!("Error checking bundle status: {:?}", error);
         } else {
             println!("Unexpected response format. Waiting...");
         }
 
         if attempt < max_retries {
             sleep(retry_delay).await;
         }
     }

    Ok("sent".to_string())
}


async fn check_final_bundle_status(jito_sdk: &JitoJsonRpcSDK, bundle_uuid: &str) -> Result<()> {
    let max_retries = 10;
    let retry_delay = Duration::from_secs(2);

    for attempt in 1..=max_retries {
        println!("Checking final bundle status (attempt {}/{})", attempt, max_retries);

        let status_response = jito_sdk.get_bundle_statuses(vec![bundle_uuid.to_string()]).await?;
        let bundle_status = get_bundle_status(&status_response)?;

        match bundle_status.confirmation_status.as_deref() {
            Some("confirmed") => {
                println!("Bundle confirmed on-chain. Waiting for finalization...");
                check_transaction_error(&bundle_status)?;
            },
            Some("finalized") => {
                println!("Bundle finalized on-chain successfully!");
                check_transaction_error(&bundle_status)?;
                print_transaction_url(&bundle_status);
                return Ok(());
            },
            Some(status) => {
                println!("Unexpected final bundle status: {}. Continuing to poll...", status);
            },
            None => {
                println!("Unable to parse final bundle status. Continuing to poll...");
            }
        }

        if attempt < max_retries {
            sleep(retry_delay).await;
        }
    }

    Err(anyhow!("Failed to get finalized status after {} attempts", max_retries))
}

fn check_transaction_error(bundle_status: &BundleStatus) -> Result<()> {
    if let Some(err) = &bundle_status.err {
        if err["Ok"].is_null() {
            println!("Transaction executed without errors.");
            Ok(())
        } else {
            println!("Transaction encountered an error: {:?}", err);
            Err(anyhow!("Transaction encountered an error"))
        }
    } else {
        Ok(())
    }
}

fn print_transaction_url(bundle_status: &BundleStatus) {
    if let Some(transactions) = &bundle_status.transactions {
        if let Some(tx_id) = transactions.first() {
            println!("Transaction URL: https://solscan.io/tx/{}", tx_id);
        } else {
            println!("Unable to extract transaction ID.");
        }
    } else {
        println!("No transactions found in the bundle status.");
    }
}

fn get_bundle_status(status_response: &serde_json::Value) -> Result<BundleStatus> {
    status_response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(|value| value.as_array())
        .and_then(|statuses| statuses.get(0))
        .ok_or_else(|| anyhow!("Failed to parse bundle status"))
        .map(|bundle_status| BundleStatus {
            confirmation_status: bundle_status.get("confirmation_status").and_then(|s| s.as_str()).map(String::from),
            err: bundle_status.get("err").cloned(),
            transactions: bundle_status.get("transactions").and_then(|t| t.as_array()).map(|arr| {
                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()
            }),
        })
}