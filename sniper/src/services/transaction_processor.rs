use crate::api::APP_STATE;
use crate::api::get_account_info_fast;
use crate::config::{BUY_ENABLED, PUBLIC_KEY, WHITELIST_ENABLED};
use crate::models::pump_parser::PumpInstructionType;
use crate::models::{PumpParser, TransactionResults};
use crate::monitor::GRPC_NORMAL;
use crate::transaction::{pump_buy, pump_sell};
use dashmap::DashMap;
use futures::stream::{FuturesUnordered, StreamExt};
use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use solana_entry::entry::Entry;
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::time::Duration;

#[derive(Debug, Clone)]
pub struct TokenState {
    pub first_buy_price: Option<f64>,
    pub current_price: Option<f64>,
    pub balance: Option<u64>,
    pub bonding_curve: Option<String>, // 可选: 用于判断是否刚发行
    pub sell_stage: u8,                // 0 = 未卖，1 = 卖过一阶段，2 = 卖过两阶段
    pub highest_price: f64,            // 👈 新增字段
    pub last_tx_time: Option<Instant>,
    pub last_tx_price: Option<f64>,
    pub token_creator: Pubkey,
    pub first_buy_time: Option<Instant>,
    // 可扩展字段: 是否卖出、狙击时间戳等
}

lazy_static! {
    pub static ref TOKEN_TABLE: DashMap<Pubkey, TokenState> = DashMap::new();
}

pub fn update_token_state<F>(mint: Pubkey, update_fn: F)
where
    F: FnOnce(&mut TokenState),
{
    let mut entry = TOKEN_TABLE.entry(mint).or_insert(TokenState {
        first_buy_price: None,
        current_price: None,
        balance: None,
        bonding_curve: None,
        sell_stage: 0,
        highest_price: 0.0,
        last_tx_time: None,
        last_tx_price: None,
        token_creator: Pubkey::default(),
        first_buy_time: None,
    });

    update_fn(&mut entry);
    println!("[🔄 TokenState已更新] {:?}", *entry);
}

pub struct TransactionProcessor;

const BATCH_SIZE: usize = 800;

pub static BLACKLIST: Lazy<Arc<RwLock<HashSet<String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashSet::new())));

pub static WHITELIST: Lazy<Arc<RwLock<HashSet<String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashSet::new())));

pub async fn watch_blacklist_txt(path: &'static str) {
    use std::time::Duration;
    use tokio::{fs, time};

    loop {
        if let Ok(content) = fs::read_to_string(path).await {
            let mut new_set = HashSet::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    new_set.insert(trimmed.to_string());
                }
            }

            let mut bl = BLACKLIST.write().await;
            *bl = new_set;

            println!("[狗庄黑名单] 已更新, 当前 {} 个地址", bl.len());
        }

        time::sleep(Duration::from_secs(60)).await;
    }
}

pub async fn watch_whitelist_txt(path: &'static str) {
    use std::time::Duration;
    use tokio::time;

    loop {
        read_whitelist(path).await;
        time::sleep(Duration::from_secs(50)).await;
    }
}

async fn read_whitelist(path: &str) {
    use tokio::fs;

    if let Ok(content) = fs::read_to_string(path).await {
        let mut new_set = HashSet::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                new_set.insert(trimmed.to_string());
            }
        }

        let mut wl = WHITELIST.write().await;
        *wl = new_set;

        println!("[狗庄白名单] 已更新, 当前 {} 个地址", wl.len());
    }
}

impl TransactionProcessor {
    pub fn print_results(results: &TransactionResults) {
        // println!("Pump交易数量: {}", results.pump_transactions.len());
        for tx in &results.pump_transactions {
            let tx_str = format!("{}", tx);
            if !tx_str.trim().is_empty() {
                // println!("--------------------------------------------------------");
                // println!("slot: {}", results.current_slot);
                // println!("{}", tx_str);
                // println!("--------------------------------------------------------");
            }
        }
    }

    pub async fn process_entries(entries: &[Entry], slot: u64) -> TransactionResults {
        unsafe {
            if !*BUY_ENABLED {
                return TransactionResults::new();
            }
        }

        if !GRPC_NORMAL.load(std::sync::atomic::Ordering::Relaxed) {
            return TransactionResults::new();
        }

        let total_txs = entries.iter().map(|e| e.transactions.len()).sum::<usize>();
        let mut all_transactions = Vec::with_capacity(total_txs);

        let results = Arc::new(Mutex::new(TransactionResults::new()));
        results.lock().await.set_current_slot(slot);

        for entry in entries {
            all_transactions.extend_from_slice(&entry.transactions);
        }

        let mut futures = FuturesUnordered::new();

        for chunk in all_transactions.chunks(BATCH_SIZE) {
            let chunk = chunk.to_vec(); // Clone chunk to move into task
            let results = Arc::clone(&results);

            futures.push(tokio::spawn(async move {
                let mut batch_results = Vec::with_capacity(chunk.len() / 20);

                for tx in chunk {
                    if let Some(pump_tx) = PumpParser::parse_transaction(&tx) {
                        let has_create = pump_tx
                            .instructions
                            .iter()
                            .any(|ix| matches!(ix.instruction_type, PumpInstructionType::Create));
                        let has_buy = pump_tx
                            .instructions
                            .iter()
                            .any(|ix| matches!(ix.instruction_type, PumpInstructionType::Buy));

                        let blacklist = BLACKLIST.read().await;
                        if blacklist.contains(pump_tx.creator.as_str()) || blacklist.contains("all")
                        {
                            continue;
                        }

                        unsafe {
                            if *WHITELIST_ENABLED  {
                                let whitelist = WHITELIST.read().await;
                                if !whitelist.contains(pump_tx.creator.as_str()) {
                                    continue;
                                }
                            }
                        }

                        if has_create
                            && has_buy
                            && (300_000_000..=7_000_000_000).contains(&pump_tx.max_sol_cost)
                        {
                            let Ok(mint) = Pubkey::from_str(&pump_tx.mint) else {
                                continue;
                            };
                            let Ok(bonding_curve) = Pubkey::from_str(&pump_tx.bonding_curve) else {
                                continue;
                            };
                            let Ok(associated_bonding_curve) =
                                Pubkey::from_str(&pump_tx.associated_bonding_curve)
                            else {
                                continue;
                            };
                            let Ok(creator) = Pubkey::from_str(&pump_tx.creator) else {
                                continue;
                            };

                            // 并行执行 buy 和 sell
                            let buy_result = pump_buy(
                                mint,
                                bonding_curve,
                                associated_bonding_curve,
                                creator,
                                slot,
                                pump_tx.price,
                                pump_tx.my_token_amount,
                            )
                                .await;

                            tokio::time::sleep(Duration::from_millis(1500)).await;
                            let wallet_pubkey =
                                Pubkey::from_str(PUBLIC_KEY.as_str()).unwrap();

                            let ata = get_associated_token_address(&wallet_pubkey, &mint);
                            let app_state = APP_STATE.get().expect("AppState not initialized");
                            let rpc_client = &app_state.client;

                            if let Ok(account) =
                                get_account_info_fast(&rpc_client, &mint, &ata).await
                            {
                                let balance = account.base.amount;

                                if balance > 0 {
                                    println!(
                                        "{}: {}, {}: {}, {}: {},",
                                        "🎯狙击成功",
                                        mint,
                                        "当前余额为",
                                        balance.to_string(),
                                        "购买成本价",
                                        pump_tx.price
                                    );
                                    // update_price_once(mint.clone().to_string(), pump_tx.price).await;
                                    // insert_address(bonding_curve.to_string()).await;
                                    update_token_state(mint.clone(), |state| {
                                        if state.first_buy_price.is_none() {
                                            state.first_buy_price = Some(pump_tx.price);
                                        }
                                        if state.current_price.is_none() {
                                            state.current_price = Some(pump_tx.price);
                                        }
                                        if state.balance.is_none() {
                                            state.balance = Some(balance);
                                        }
                                        if state.bonding_curve.is_none() {
                                            state.bonding_curve = Some(bonding_curve.to_string());
                                        }
                                        let now = Instant::now();
                                        if state.last_tx_time.is_none() {
                                            state.last_tx_time = Some(now);
                                        }
                                        if state.last_tx_price.is_none() {
                                            state.last_tx_price = Some(pump_tx.price);
                                        }

                                        state.token_creator = creator;

                                        if state.first_buy_time.is_none() {
                                            state.first_buy_time = Some(now);
                                        }
                                    });

                                    // ✅ 在这之后启动4秒止损监测任务
                                    let mint_clone = mint.clone();
                                    tokio::spawn(async move {
                                        tokio::time::sleep(Duration::from_millis(2000)).await;

                                        println!("[🔻开始判断3.5秒止损] {}, [{:?}]", mint_clone, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                                        if let Some(state) = TOKEN_TABLE.get(&mint_clone) {
                                            let first = state.first_buy_price.unwrap_or(0.0);
                                            let current = state.current_price.unwrap_or(0.0);
                                            let creator_pubkey = creator.clone();
                                            drop(state); // ✅ 显式释放锁，避免与 remove 冲突

                                            let change = ((current - first) / first).abs();

                                            if change < 0.20 {
                                                println!("[🔻3.5秒止损触发] {} 当前价: {:.12}, 原价: {:.12}, 变动: {:.2}%, [{:?}]", mint_clone, current, first, change * 100.0,  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

                                                if let Err(e) = pump_sell(
                                                    mint_clone.clone(),
                                                    creator_pubkey,
                                                    balance,
                                                )
                                                    .await
                                                {
                                                    println!("[❌止损失败] {:?}", e);
                                                } else {
                                                    TOKEN_TABLE.remove(&mint_clone); // ✅ 现在不会死锁
                                                    println!(
                                                        "[✅止损成功] 已卖出代币 {}, [{:?}]",
                                                        mint_clone, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
                                                    );
                                                }
                                            } else {
                                                println!("[✅无需止损] {} 价格已涨 {:.2}%，未触发3.5秒止损。", mint_clone, change * 100.0);
                                            }
                                        }
                                    });
                                } else {
                                    println!("查询ATA失败（可能不存在或错误），不卖出。");
                                }
                            }

                            batch_results.push(pump_tx);
                        }
                    }
                }

                if !batch_results.is_empty() {
                    let mut results_lock = results.lock().await;
                    results_lock.pump_transactions.reserve(batch_results.len());
                    for tx in batch_results {
                        results_lock.add_pump_transaction(tx);
                    }
                }
            }));
        }

        // 等待所有异步任务完成
        while let Some(res) = futures.next().await {
            if let Err(e) = res {
                eprintln!("Batch task failed: {:?}", e);
            }
        }

        Arc::try_unwrap(results).unwrap().into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::get_rpc_client;
    use solana_client::rpc_config::RpcTransactionConfig;
    use solana_sdk::commitment_config::CommitmentConfig;
    use solana_sdk::signature::Signature;
    use solana_transaction_status::UiTransactionEncoding;

    #[tokio::test]
    pub async fn test_process_entries() {
        dotenvy::dotenv().ok();

        read_whitelist("whitelist.txt").await;

        let client = get_rpc_client().unwrap();

        let signature = Signature::from_str("gKyEu81Zqh5DiiQynMzKayvmXwxtrtEywFazCieKpAiiEh5a6GLk5kr4GEGa3xa1nHoZGzkYUBgsUo1UUYdPYZS").unwrap();
        let tx = client
            .get_transaction_with_config(
                &signature,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Base64),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )
            .await
            .unwrap();
        let tx = tx.transaction.transaction.decode().unwrap();

        let entry = Entry {
            num_hashes: 0,
            hash: Default::default(),
            transactions: vec![tx],
        };

        TransactionProcessor::process_entries(&vec![entry], 1).await;
    }
}
