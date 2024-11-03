use base64::Engine;
use solana_program::system_program;
use crate::modules::instruction::compile_instruction;
use serde_json::json;
use jito_sdk_rust::JitoJsonRpcSDK;
use solana_sdk::{instruction::CompiledInstruction, system_instruction};
use crate::turnkey::errors::TurnkeyError;
use {
    super::matis::SwapTransaction, crate::turnkey::{
        client::{KeyInfo, Turnkey}, errors::TurnkeyResult
    }, serde::{Deserialize, Serialize}, solana_client::rpc_client::RpcClient, solana_sdk::{
        pubkey::Pubkey, signature::Signature, transaction::Transaction
    }, std::{env, str::FromStr}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub api_public_key: String,
    pub api_private_key: String,
    pub organization_id: String,
    pub public_key: String,
}

pub async fn sign_and_send_swap_transaction(transaction: SwapTransaction, user: User, jito_tip_amount: u64) -> TurnkeyResult<Signature> {
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

    let jito_sdk = JitoJsonRpcSDK::new(env::var("JITO_BLOCK_ENGINE_URL").expect("JITO_BLOCK_ENGINE_URL must be set").as_str(), None);
    let mut transaction = match bincode::deserialize::<Transaction>(&transaction_data) {
        Ok(mut tx) => {
            // Attempt to get a random tip account
            match jito_sdk.get_random_tip_account().await {
                Ok(random_tip_account) => {
                    println!("@solana_app/modules/swap/sign_and_send_swap_transaction/ random_tip_account: {}", random_tip_account);
                    let jito_tip_account = Pubkey::from_str(&random_tip_account).expect("Failed to parse random tip account");

                    let data = system_instruction::transfer(
                        &pubkey,
                        &jito_tip_account,
                        jito_tip_amount,
                    ).data;

                    //  create a compiled ix to tip transfer
                    let account_keys  = vec![pubkey, jito_tip_account, system_program::id()];
                    let compiled_ix = CompiledInstruction {
                        program_id_index: account_keys.iter().position(|key| key == &system_program::id()).unwrap() as u8,
                        accounts: account_keys.iter().map(|key| account_keys.iter().position(|k| k == key).unwrap() as u8).collect(),
                        data
                    };

                    tx.message.instructions.push(compiled_ix);
                    if !tx.message.account_keys.contains(&jito_tip_account) {
                        tx.message.account_keys.push(jito_tip_account.clone());
                    }


                    let key_info = KeyInfo {
                        private_key_id: public_key.to_string(),
                        public_key: pubkey
                    };

                    match turnkey_client.sign_transaction(&mut tx, key_info).await {
                        Ok((signed_tx, _sig)) => {
                            // Create the tip parameters for Jito
                            let serialized_tx = engine.encode(bincode::serialize(&signed_tx).unwrap());
                            let jito_params = json!({
                                "tx": serialized_tx
                            });

                            // Send transaction to Jito block engine
                            match jito_sdk.send_txn(Some(jito_params), true).await {
                                Ok(tx_sig) => {
                                    println!("@sign_and_send_swap_transaction/ transaction sent to Jito Engine: {:?}", tx_sig);
                                },
                                Err(e) => {
                                    println!("@sign_and_send_swap_transaction/ error sending transaction to Jito Engine: {:?}", e);
                                }
                            }
                        },
                        Err(e) => {
                            println!("Failed to sign transaction: {:?}", e);
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to get random tip account: {:?}", e);
                    return Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to get random tip account: {:?}", e))));
                }
            }

            println!("@sign_and_send_swap_transaction/ transaction deserialized successfully");
            println!("@sign_and_send_swap_transaction/ transaction: {:?}", tx);
            Some(tx)
        },
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
                        let jito_params = json!({
                            "tx": tx_sig.to_string()
                        });
                        match jito_sdk.send_txn(Some(jito_params), true).await {
                            Ok(tx_sig) => {
                                println!("@sign_and_send_swap_transaction/ transaction sent to jito Engine: {:?}", tx_sig);
                            },
                            Err(e) => {
                                println!("@sign_and_send_swap_transaction/ error sending transaction to jito Engine: {:?}", e);
                                return Err(TurnkeyError::from(Box::<dyn std::error::Error>::from(format!("Failed to send transaction to jito Engine: {:?}", e))));
                            }
                        }
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
