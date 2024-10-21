use {
    super::matis::SwapTransaction,
    crate::turnkey::{
        client::{KeyInfo, Turnkey}, errors::TurnkeyResult
    }, bincode::deserialize, solana_client::rpc_client::RpcClient, solana_sdk::{
        pubkey::Pubkey,
        transaction::Transaction,
    }, std::{env, str::FromStr}
};

struct User {
    api_public_key: String,
    api_private_key: String,
    organization_id: String,
    public_key: String,
}


async fn sign_swap_transaction(transaction: SwapTransaction, user: User) -> TurnkeyResult<()> {
    // Initialize Turnkey client
    let turnkey_client = Turnkey::new_for_user(&user.api_public_key, &user.api_private_key, &user.organization_id, &user.public_key)?;
    let pubkey = Pubkey::from_str(&user.public_key).expect("Invalid pubkey");

    // Initialize RPC client
    let rpc_client = RpcClient::new(env::var("NODE_HTTP").expect("NODE_HTTP must be set"));

    // Decode transaction
    let transaction_data = base64::decode(transaction.swap_transaction).expect("Invalid transaction data");
    let mut transaction = deserialize::<Transaction>(&transaction_data[..]).expect("Invalid transaction");

    // Get latest blockhash
    let key_info = KeyInfo {
       private_key_id: user.public_key,
       public_key: pubkey
    };

    // Sign transaction
    println!("Signing transaction");
    let (tx, _sig) = turnkey_client.sign_transaction(&mut transaction, key_info).await?;
    println!("Transaction signed");

    println!("Sending transaction");
    let tx_sig = rpc_client.send_and_confirm_transaction(&tx).expect("Failed to send transaction");
    println!("Transaction confirmed: {:?}", tx_sig);

    Ok(())

}