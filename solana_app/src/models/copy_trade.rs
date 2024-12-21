/// Struct representing a copy trade wallet
#[derive(Debug, serde::Deserialize, Clone)]
pub struct CopyTradeWallet {
    pub copy_trade_address: String,
    pub account_address: String,
    pub buy_amount: f64,
    pub status: bool,
}
