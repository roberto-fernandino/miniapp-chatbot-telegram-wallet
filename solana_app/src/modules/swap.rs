use {
    super::matis::SwapTransaction, crate::turnkey::{
        client::{KeySelector, Turnkey}, errors::TurnkeyResult
    }, solana_client::rpc_client::RpcClient, solana_sdk::{
        commitment_config::CommitmentConfig, message::Message, pubkey::Pubkey,
        system_instruction, transaction::Transaction,
    }, std::{env, str::FromStr},
    solana_sdk::transaction::VersionedTransaction,
    bincode::deserialize,
};

struct User {
    api_public_key: String,
    api_private_key: String,
    organization_id: String,
    public_key: String,
}

async fn sign_swap_transaction(transaction: SwapTransaction, user: User) -> TurnkeyResult<()> {
    let turnkey_client = Turnkey::new_for_user(&user.api_public_key, &user.api_private_key, &user.organization_id, &user.public_key)?;
    let rpc_client = RpcClient::new(env::var("NODE_HTTP").expect("NODE_HTTP must be set"));
    let pubkey = Pubkey::from_str(&user.public_key).expect("Invalid pubkey");
    let transaction_data = base64::decode(transaction.swap_transaction).expect("Invalid transaction data");
    let transaction = deserialize::<VersionedTransaction>(&transaction_data[..]).expect("Invalid transaction");
    let recent_blockhash = rpc_client.get_latest_blockhash().expect("Failed to get latest blockhash");

    // let (tx, _sig) = turnkey_client.sign_transaction(transaction, KeySelector::ExampleKey, recent_blockhash);
    Ok(())

}