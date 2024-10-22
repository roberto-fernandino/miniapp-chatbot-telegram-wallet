use base64::engine::general_purpose;
use anyhow::Result;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use solana_client::rpc_client::RpcClient;
use solana_sdk::native_token::sol_to_lamports;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::VersionedTransaction;
use std::env;
use std::error::Error;
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Instant;

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFee {
    pub amount: String,
    pub fee_bps: Number,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: Number,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: Number,
    pub platform_fee: PlatformFee,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlan>,
    pub context_slot: u64,
    pub time_taken: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapTransaction {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
    pub prioritization_fee_lamports: u64,
}
/// Get a quote for a swap
/// # Arguments
/// * `input_mint` - The mint of the token to be swapped
/// * `output_mint` - The mint of the token to be received
/// * `amount` - The amount of the token to be swapped
///
pub async fn get_versioned_quote(
    input_mint: String,
    output_mint: String,
    amount: String,
    slippage: f64
) -> Result<Quote> {
    // Create a new reqwest client
    let client = reqwest::Client::new();
    let slippage_bps = (slippage * 100.0).round() as u64;
    // Setup the url
    let url = format!(
        "{}/quote?inputMint={input_mint}&outputMint={output_mint}&amount={amount}&slippageBps={slippage_bps}",
        env::var("METIS_HTTP").expect("METIS_HTTP must be set")
    );

    // Create the request
    let request: reqwest::RequestBuilder = client.request(reqwest::Method::GET, url);

    // Send the request
    let response = request.send().await?;
    let body = response.text().await?;
    let quote = deserialize_quote(&body)?;

    Ok(quote)
}


/// Get a legacy quote for a swap
/// # Arguments
/// * `input_mint` - The mint of the token to be swapped
/// * `output_mint` - The mint of the token to be received
/// * `amount` - The amount of the token to be swapped
/// * `slippage` - The slippage tolerance
pub async fn get_legacy_quote(
    input_mint: String,
    output_mint: String,
    amount: String,
    slippage: f64
) -> Result<Quote> {
    // Create a new reqwest client
    let client = reqwest::Client::new();
    let slippage_bps = (slippage * 100.0).round() as u64;
    // Setup the url
    let url = format!(
        "{}/quote?inputMint={input_mint}&outputMint={output_mint}&amount={amount}&slippageBps={slippage_bps}&asLegacyTransaction=true",
        env::var("METIS_HTTP").expect("METIS_HTTP must be set")
    );

    // Create the request
    let request: reqwest::RequestBuilder = client.request(reqwest::Method::GET, url);

    // Send the request
    let response = request.send().await?;
    let body = response.text().await?;
    let quote = deserialize_quote(&body)?;

    Ok(quote) 
}


/// Get a legacy swap transaction
/// # Arguments
/// * `user_public_key` - The public key of the user
/// * `priorization_fee_lamports` - The priorization fee in lamports
/// * `input_mint` - The mint of the token to be swapped
/// * `output_mint` - The mint of the token to be received
/// * `amount` - The amount of the token to be swapped
/// * `slippage` - The slippage tolerance
///
/// # Returns
/// * `SwapTransaction` - The swap transaction
pub async fn get_legacy_swap_transaction(
    user_public_key: &Pubkey,
    priorization_fee_lamports: u64,
    input_mint: String,
    output_mint: String,
    amount: u64,
    slippage: f64
) -> Result<SwapTransaction> {
 let client = reqwest::Client::new();
    let url = format!(
        "{}/swap",
        env::var("METIS_HTTP").expect("METIS_HTTP must be set")
    );
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse()?);
    let quote = get_legacy_quote(input_mint, output_mint, amount.to_string(), slippage).await.expect("Failed to get quote");

    let data = format!(
        r#"{{
        "userPublicKey": "{}",
        "priorizationFeeLamports": {},
        "quoteResponse": {},
        "asLegacyTransaction": true
    }}"#,
        user_public_key.to_string(),
        priorization_fee_lamports,
        serialize_quote(quote)?
    );
    let json: serde_json::Value = serde_json::from_str(&data)?;

    let request = client
        .request(reqwest::Method::POST, url)
        .headers(headers)
        .json(&json);

    let response = request.send().await?;
    let body = response.text().await?;
    let swap_transaction: SwapTransaction = serde_json::from_str(&body)?;

    Ok(swap_transaction)
}

fn deserialize_quote(quote: &str) -> Result<Quote> {
    let quote: Quote = serde_json::from_str(quote)?;
    Ok(quote)
}

fn serialize_quote(quote: Quote) -> Result<String> {
    let quote = serde_json::to_string(&quote)?;
    Ok(quote)
}

/// Get a versioned swap transaction
/// # Arguments
/// * `user_public_key` - The public key of the user
/// * `priorization_fee_lamports` - The priorization fee in lamports
/// * `input_mint` - The mint of the token to be swapped
/// * `output_mint` - The mint of the token to be received
/// * `amount` - The amount of the token to be swapped
/// * `slippage` - The slippage tolerance
/// # Returns
/// * `SwapTransaction` - The swap transaction
pub async fn get_swap_versioned_transaction(
    user_public_key: &Pubkey,
    priorization_fee_lamports: u64,
    input_mint: String,
    output_mint: String,
    amount: u64,
    slippage: f64
) -> Result<SwapTransaction>{
    let client = reqwest::Client::new();
    let url = format!(
        "{}/swap",
        env::var("METIS_HTTP").expect("METIS_HTTP must be set")
    );
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse()?);
    let quote = get_versioned_quote(input_mint, output_mint, amount.to_string(), slippage).await.expect("Failed to get quote");

    let data = format!(
        r#"{{
        "userPublicKey": "{}",
        "priorizationFeeLamports": {},
        "quoteResponse": {} 
    }}"#,
        user_public_key.to_string(),
        priorization_fee_lamports,
        serialize_quote(quote)?
    );
    let json: serde_json::Value = serde_json::from_str(&data)?;

    let request = client
        .request(reqwest::Method::POST, url)
        .headers(headers)
        .json(&json);

    let response = request.send().await?;
    let body = response.text().await?;
    let swap_transaction: SwapTransaction = serde_json::from_str(&body)?;

    Ok(swap_transaction)
}

pub async fn send_buy_swap_transaction(
    keypair: &Keypair,
    client: &RpcClient,
    mint: String,
    amount: f64,
) -> Result<Signature, Box<dyn Error>> {
    let start = Instant::now();
    
    // Aumentar a taxa de priorização
    let swap_transaction = get_swap_versioned_transaction(
        &keypair.pubkey(),
        sol_to_lamports(0.05), // Aumentado de 0.03 para 0.1
        SOL_MINT.to_string(),
        mint,
        sol_to_lamports(amount),
        0.18
    )
    .await?;

    // Decode the transaction 
    let transaction_bytes = general_purpose::STANDARD.decode(&swap_transaction.swap_transaction)?;
    let mut transaction: VersionedTransaction = bincode::deserialize(&transaction_bytes)?;

    // Assign the signature to the transaction
    let signature = keypair.sign_message(&transaction.message.serialize());
    transaction.signatures[0] = signature;

    // Send the transaction without waiting for confirmation
    let signature = client.send_transaction(&transaction)?;

    // Implement a custom confirmation logic
    let confirmation_start = Instant::now();
    
    let signature: Result<Signature, Box<dyn Error>> = loop {
        if let Ok(response) = client.confirm_transaction_with_commitment(&signature, CommitmentConfig::confirmed()) {
            if response.value {
                println!("Transaction confirmed in {:?}", confirmation_start.elapsed());
                break Ok(signature);
            }
        }
    };
    Ok(signature.expect("Signatrue not found."))
}

pub async fn send_sell_swap_transaction(
    keypair: &Keypair,
    client: &RpcClient,
    mint: String,
    amount: u64,
) -> Result<()> {
    let start = chrono::Utc::now();
    // Get the swap transaction
    let swap_transaction = get_swap_versioned_transaction(
        &keypair.pubkey(),
        sol_to_lamports(0.05),
        mint.to_string(),
        SOL_MINT.to_string(),
        amount,
        0.18
    )
    .await?;

    println!(
        "Get swap time taken: {}",
        chrono::Utc::now().signed_duration_since(start)
    );

    let start = chrono::Utc::now();
    // Decode the swap transaction
    let transaction_bytes =
        match general_purpose::STANDARD.decode(swap_transaction.swap_transaction) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Failed to decode swap transaction: {:?}", e);
                return Err(anyhow::anyhow!("Failed to decode swap transaction: {:?}", e));
            }
        };

    // Deserialize the transaction
    let mut transaction: VersionedTransaction =
        match bincode::deserialize_from(&mut transaction_bytes.as_slice()) {
            Ok(tx) => tx,
            Err(e) => {
                eprintln!("Failed to deserialize transaction: {:?}", e);
                eprintln!("Bytes length: {}", transaction_bytes.len());
                return Err(anyhow::anyhow!("Failed to deserialize transaction: {:?}", e));
            }
        };

    let signature = keypair.sign_message(&transaction.message.serialize());
    let mut signatures = transaction.signatures.clone();
    signatures[0] = signature; // Substitui a assinatura vazia pela nova
    transaction.signatures = signatures;
    let result = client.send_and_confirm_transaction(&transaction)?;
    println!("result: {}", result);
    println!(
        "Deserialize send transaction time taken: {}",
        chrono::Utc::now().signed_duration_since(start)
    );
    Ok(())
}
