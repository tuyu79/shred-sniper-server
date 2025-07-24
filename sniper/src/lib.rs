use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::env;
use std::sync::Arc;

pub mod api;
pub mod config;
pub mod models;
pub mod monitor;
pub mod services;
pub mod transaction;
pub mod tx;
pub mod utils;
mod server;

// 重新导出重要的类型，方便调用
pub use api::{AppState, APP_STATE};
pub use models::TransactionResults;
pub use services::{JitoClient, TransactionProcessor};

pub fn get_rpc_client() -> Result<Arc<RpcClient>> {
    let rpc_url = env::var("RPC_ENDPOINTS")?; // 直接获取环境变量
    let client = RpcClient::new(rpc_url);
    return Ok(Arc::new(client));
}
