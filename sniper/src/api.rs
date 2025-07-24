use anyhow::Result;
use once_cell::sync::OnceCell;
use solana_account_decoder::UiAccountData;
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use spl_token_2022::{
    extension::StateWithExtensionsOwned,
    state::{Account, Mint},
};
use spl_token_client::{
    client::{ProgramRpcClient, ProgramRpcClientSendTransaction}, // 加回 SendTransaction
    token::{TokenError, TokenResult},                            // 正确导入 TokenResult
};
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
#[derive(Clone)]
pub struct AppState {
    pub client: Arc<RpcClient>,
}

// 手动实现 Debug（避免 RpcClient 无法自动推导）
impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("client", &"RpcClient(...)")
            .finish()
    }
}

// 全局变量（只能 set 一次）
pub static APP_STATE: OnceCell<Arc<AppState>> = OnceCell::new();

pub async fn get_account_info_fast(
    client: &Arc<RpcClient>,
    address: &Pubkey, // 代币mint地址
    ata: &Pubkey,     // 用户的ATA地址
) -> TokenResult<StateWithExtensionsOwned<Account>> {
    let config = RpcAccountInfoConfig {
        commitment: Some(CommitmentConfig::processed()), // 用最快的 commitment
        encoding: Some(UiAccountEncoding::Base64Zstd),
        data_slice: None,
        min_context_slot: None,
    };

    let account = client
        .get_account_with_config(ata, config)
        .await
        .map_err(|e| TokenError::Client(e.into()))?
        .value // 👈 这里拿出 Option<Account>
        .ok_or(TokenError::AccountNotFound)
        .inspect_err(|err| warn!("{} {}: mint {}", ata, err, address))?;

    // 校验账户属于spl-token program
    if account.owner != spl_token::ID {
        return Err(TokenError::AccountInvalidOwner);
    }

    // 解码账户内容
    let account = StateWithExtensionsOwned::<Account>::unpack(account.data)
        .map_err(|_| TokenError::AccountInvalidMint)?;

    // 校验账户mint是否匹配
    if account.base.mint != *address {
        return Err(TokenError::AccountInvalidMint);
    }

    Ok(account)
}

pub fn get_rpc_client() -> Result<Arc<RpcClient>> {
    let rpc_url = env::var("RPC_ENDPOINTS")?; // 直接获取环境变量
    let client = RpcClient::new(rpc_url);
    return Ok(Arc::new(client));
}
