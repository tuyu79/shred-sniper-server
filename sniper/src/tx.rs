use anyhow::{Context, Error, Result, anyhow};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, str::FromStr, sync::Arc, time::Duration};
// use solana_client::rpc_client::RpcClient;
use solana_client::nonblocking::rpc_client::RpcClient; // 非阻塞版
use solana_rpc_client_nonce_utils::nonblocking;
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::{Message, VersionedMessage},
    nonce::state::State as NonceState,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::VersionedTransaction,
};

use base64::{Engine as _, engine::general_purpose};
use bincode;
use rand::Rng;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use solana_sdk::commitment_config::CommitmentConfig;
use tokio::{
    sync::{Mutex, OnceCell, RwLock},
    time::{Instant, sleep},
};
use tracing::{error, info, warn};

use crate::api::APP_STATE;
use crate::api::AppState;
use crate::config::{
    JITO_FEE, JITO_RPC_ENDPOINTS, NONCE_PUBKEY, ZERO_SLOT_BUY_FEE, ZERO_SLOT_RPC_ENDPOINTS,
    ZERO_SLOT_SELL_FEE,
};
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::pubkey;
use solana_sdk::transaction::Transaction;

lazy_static::lazy_static! {
    pub static ref RECENT_BLOCKHASH: Arc<RwLock<Hash>> = Arc::new(RwLock::new(Hash::default()));
    pub static ref SELL_RECENT_BLOCKHASH: Arc<RwLock<Hash>> = Arc::new(RwLock::new(Hash::default()));
}
lazy_static::lazy_static! {
    static ref JITO_TIP_ACCOUNTS: [Pubkey; 8] = [
        Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap(),
        Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe").unwrap(),
        Pubkey::from_str("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY").unwrap(),
        Pubkey::from_str("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49").unwrap(),
        Pubkey::from_str("DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh").unwrap(),
        Pubkey::from_str("ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt").unwrap(),
        Pubkey::from_str("DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL").unwrap(),
        Pubkey::from_str("3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT").unwrap(),
    ];

    static ref ZEROSLOT_TIP_ACCOUNTS: [Pubkey; 5] = [
        Pubkey::from_str("Eb2KpSC8uMt9GmzyAEm5Eb1AAAgTjRaXWFjKyFXHZxF3").unwrap(),
        Pubkey::from_str("FCjUJZ1qozm1e8romw216qyfQMaaWKxWsuySnumVCCNe").unwrap(),
        Pubkey::from_str("ENxTEjSQ1YabmUpXAdCgevnHQ9MHdLv8tzFiuiYJqa13").unwrap(),
        Pubkey::from_str("6rYLG55Q9RpsPGvqdPNJs4z5WTxJVatMB8zV3WJhs5EK").unwrap(),
        Pubkey::from_str("Cix2bHfqPcKcM233mzxbLk14kSggUUiz2A87fJtGivXr").unwrap(),
    ];
}

lazy_static::lazy_static! {
    static ref TEM_TEMPLATE: serde_json::Value = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": [
            "",  // 这里占位，后续替换成 base64
            { "encoding": "base64" }
        ]
    });
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::builder()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Content-Type", "application/json".parse().unwrap());
            headers
        })
        .pool_idle_timeout(None) // 永不关闭空闲连接
        .pool_max_idle_per_host(10) // 每个主机最多 10 个空闲连接
        .timeout(Duration::from_millis(500)) // 超时 500ms
        .build()
        .unwrap();
}

fn get_random_tip_account() -> Pubkey {
    let mut rng = rand::thread_rng();
    JITO_TIP_ACCOUNTS[rng.gen_range(0..8)]
}

fn get_0slot_tip_account() -> Pubkey {
    let mut rng = rand::thread_rng();
    ZEROSLOT_TIP_ACCOUNTS[rng.gen_range(0..5)]
}

fn extract_blockhash(nonce_state: NonceState) -> Result<Hash> {
    match nonce_state {
        NonceState::Initialized(data) => Ok(data.blockhash()),
        _ => Err(anyhow!("Nonce account not initialized")),
    }
}

// 获取 nonce 账户状态
async fn get_nonce_state(rpc: &RpcClient, nonce_pubkey: &Pubkey) -> Result<NonceState> {
    let nonce_account = rpc
        .get_account_with_commitment(&nonce_pubkey, CommitmentConfig::processed())
        .await?
        .value
        .ok_or_else(|| anyhow!("Nonce account not found"))?;
    let nonce_data = solana_rpc_client_nonce_utils::data_from_account(&nonce_account)?; // 解析数据
    Ok(NonceState::Initialized(nonce_data))
}

async fn first_get_nonce_state(rpc: &RpcClient, nonce_pubkey: &Pubkey) -> Result<NonceState> {
    let nonce_account = rpc
        .get_account_with_commitment(&nonce_pubkey, CommitmentConfig::finalized())
        .await?
        .value
        .ok_or_else(|| anyhow!("Nonce account not found"))?;
    let nonce_data = solana_rpc_client_nonce_utils::data_from_account(&nonce_account)?; // 解析数据
    Ok(NonceState::Initialized(nonce_data))
}

pub async fn update_nonce(state: &AppState, nonce_pubkey: Arc<Pubkey>) -> Result<()> {
    let client = state.client.clone();
    let nonce_state = get_nonce_state(&client, &nonce_pubkey).await?;
    let new_blockhash = extract_blockhash(nonce_state)?;

    let mut blockhash_lock = RECENT_BLOCKHASH.write().await;
    *blockhash_lock = new_blockhash;

    println!("手动更新 RECENT_BLOCKHASH 为: {}", new_blockhash);
    Ok(())
}

// 启动 blockhash 更新器
pub async fn start_blockhash_fetcher(state: &AppState, nonce_pubkey: Arc<Pubkey>) -> Result<()> {
    let client = state.client.clone();
    let nonce_state = first_get_nonce_state(&client, &nonce_pubkey).await?;
    let blockhash = extract_blockhash(nonce_state)?;

    let mut blockhash_lock = RECENT_BLOCKHASH.write().await;
    *blockhash_lock = blockhash;

    println!("初始化 RECENT_BLOCKHASH 地址: {}", blockhash);
    Ok(())
}

pub async fn keep_alive_loop() {
    let url = ZERO_SLOT_RPC_ENDPOINTS.as_str();
    loop {
        match HTTP_CLIENT.get(url).send().await {
            Ok(resp) => {
                if let Ok(text) = resp.text().await {
                    println!("Keep-alive OK: {}", text);
                }
            }
            Err(e) => {
                println!("Keep-alive failed: {:?}", e);
            }
        }
        sleep(Duration::from_secs(60)).await;
    }
}

pub async fn tx_pump_buy(
    keypair: &Keypair,
    mut instructions: Vec<Instruction>,
) -> Result<Vec<String>> {
    let nonce_pubkey = Pubkey::from_str(NONCE_PUBKEY.as_str()).unwrap();

    let unit_limit = 77000;
    // let unit_price = 193464;

    //unit_limit / 100,000,000
    //unit_price / 1,000,000 得出lamports
    // Set0.05 lamports per compute unit
    // 0.0007 * 0.05 = 0.000035 SOL;

    let recent_blockhash = *RECENT_BLOCKHASH.read().await;

    let instr_advance_nonce_account =
        system_instruction::advance_nonce_account(&nonce_pubkey, &keypair.pubkey());
    instructions.insert(0, instr_advance_nonce_account);

    // // 通用指令：Compute Unit 和 Priority Fee
    let modify_compute_units =
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(unit_limit);
    // let add_priority_fee =
    //     solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(unit_price);
    instructions.insert(2, modify_compute_units);
    // instructions.insert(3, add_priority_fee);

    // 并发发送交易
    tokio::join!(
        // ---------------------- 0Slot HTTP -------------------------
        async {
            let mut slot_tip = 0.0;
            unsafe {
                slot_tip = *ZERO_SLOT_BUY_FEE; // 0.001 SOLÏ
            }

            // Tip 指令（添加到0slot）
            let slot_tip_lamports = (slot_tip * 1_000_000_000.0) as u64;
            let tip_account = get_0slot_tip_account();
            let tip_instruction =
                system_instruction::transfer(&keypair.pubkey(), &tip_account, slot_tip_lamports);
            // println!("Tip account: {}, lamports: {}", tip_account, tip_lamports);

            let mut tip_instructions = instructions.clone();
            tip_instructions.insert(1, tip_instruction);

            let message = Message::new_with_blockhash(
                &tip_instructions,
                Some(&keypair.pubkey()),
                &recent_blockhash,
            );
            let versioned_message = VersionedMessage::Legacy(message);
            let versioned_tx = VersionedTransaction {
                signatures: vec![keypair.sign_message(&versioned_message.serialize())],
                message: versioned_message,
            };

            // 发送交易
            let serialized_txn = bincode::serialize(&versioned_tx)?;
            let base64_txn = general_purpose::STANDARD.encode(&serialized_txn);

            let start_send = Instant::now();
            let mut request_body = TEM_TEMPLATE.clone();
            if let Some(params) = request_body
                .get_mut("params")
                .and_then(|p| p.as_array_mut())
            {
                params[0] = serde_json::Value::String(base64_txn);
            }

            let response = HTTP_CLIENT
                .post(ZERO_SLOT_RPC_ENDPOINTS.as_str())
                .json(&request_body)
                .send()
                .await?;
            let send_duration = start_send.elapsed();
            println!(
                "Send to 0slot took {:?}, [{}]",
                send_duration,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );

            // let response_json: serde_json::Value = response.json().await?;
            // println!("0slot回应 {:?}", response_json);
            Ok::<(), anyhow::Error>(())
        },
        //---------------------- JITO HTTP -------------------------
        async {
            let mut jito_tip = 0.0;
            unsafe {
                jito_tip = *JITO_FEE; // 0.001 SOLÏ
            }

            let jito_tip_lamports = (jito_tip * 1_000_000_000.0) as u64;

            let jito_tip_account = get_random_tip_account();
            let jito_tip_instruction = system_instruction::transfer(
                &keypair.pubkey(),
                &jito_tip_account,
                jito_tip_lamports,
            );

            let mut jito_instructions = instructions.clone();
            jito_instructions.push(jito_tip_instruction);

            let message = Message::new_with_blockhash(
                &jito_instructions,
                Some(&keypair.pubkey()),
                &recent_blockhash,
            );
            let versioned_message = VersionedMessage::Legacy(message);
            let versioned_tx = VersionedTransaction {
                signatures: vec![keypair.sign_message(&versioned_message.serialize())],
                message: versioned_message,
            };
            // 发送交易
            let serialized_txn = bincode::serialize(&versioned_tx)?;
            let base64_txn = general_purpose::STANDARD.encode(&serialized_txn);

            let start_send = Instant::now();
            let mut request_body = TEM_TEMPLATE.clone();
            if let Some(params) = request_body
                .get_mut("params")
                .and_then(|p| p.as_array_mut())
            {
                params[0] = serde_json::Value::String(base64_txn);
            }

            let response = HTTP_CLIENT
                .post(JITO_RPC_ENDPOINTS.as_str())
                .json(&request_body)
                .send()
                .await?;
            let send_duration = start_send.elapsed();
            println!(
                "Send to jito-http took {:?}, [{}]",
                send_duration,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );

            Ok::<(), anyhow::Error>(())
        },
    );
    Ok(vec![])
}

pub async fn tx_pump_sell(
    keypair: &Keypair,
    mut instructions: Vec<Instruction>,
) -> Result<Vec<String>> {
    let app_state = APP_STATE.get().expect("AppState not initialized");
    let client = &app_state.client;
    let unit_limit = 75000;
    // let unit_price = 50000;

    let recent_blockhash = client
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to get blockhash: {:?}", e))?;

    let modify_compute_units =
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(unit_limit);
    // let add_priority_fee =
    //     solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(unit_price);
    instructions.insert(1, modify_compute_units);
    // instructions.insert(2, add_priority_fee);

    // ---------------------- 0Slot HTTP -------------------------
    let mut slot_tip = 0.0;
    unsafe {
        slot_tip = *ZERO_SLOT_SELL_FEE; // 0.001 SOLÏ
    }
    let slot_tip_lamports = (slot_tip * 1_000_000_000.0) as u64;
    let tip_account = get_0slot_tip_account();
    let tip_instruction =
        system_instruction::transfer(&keypair.pubkey(), &tip_account, slot_tip_lamports);

    let mut tip_instructions = instructions.clone();
    tip_instructions.insert(1, tip_instruction);

    let message = Message::new_with_blockhash(
        &tip_instructions,
        Some(&keypair.pubkey()),
        &recent_blockhash,
    );
    let versioned_message = VersionedMessage::Legacy(message);
    let versioned_tx = VersionedTransaction {
        signatures: vec![keypair.sign_message(&versioned_message.serialize())],
        message: versioned_message,
    };

    let serialized_txn = bincode::serialize(&versioned_tx)?;
    let base64_txn = general_purpose::STANDARD.encode(&serialized_txn);

    let start_send = Instant::now();
    let mut request_body = TEM_TEMPLATE.clone();
    if let Some(params) = request_body
        .get_mut("params")
        .and_then(|p| p.as_array_mut())
    {
        params[0] = serde_json::Value::String(base64_txn);
    }

    let response = HTTP_CLIENT
        .post(ZERO_SLOT_RPC_ENDPOINTS.as_str())
        .json(&request_body)
        .send()
        .await?;
    let send_duration = start_send.elapsed();
    println!(
        "Send to 0slot took {:?}, [{:?}]",
        send_duration,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    // let response_json: serde_json::Value = response.json().await?;
    // println!("0slot回应 {:?}", response_json);

    Ok(vec![])
}