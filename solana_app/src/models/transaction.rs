use serde::{Deserialize, Serialize};

use crate::handlers::matis::SwapTransaction;
/// Struct representing the payload for swap transactions
#[derive(Debug, Serialize)]
pub struct Payload {
    pub event_type: String,
    pub data: SwapTransaction,
}

// Struct definitions for parsing log notifications
#[derive(Debug, Deserialize)]
pub struct Context {
    pub slot: u64,
}

#[derive(Debug, Deserialize)]
pub struct Value {
    pub signature: String,
    pub err: Option<String>, // `err` can be `null`, so use Option
    pub logs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResultWrapper {
    pub context: Context,
    pub value: Value,
}

#[derive(Debug, Deserialize)]
pub struct Params {
    pub result: ResultWrapper,
    pub subscription: u64,
}

#[derive(Debug, Deserialize)]
pub struct LogsNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Params,
}
