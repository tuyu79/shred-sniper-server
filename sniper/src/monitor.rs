use crate::services::transaction_processor::BLACKLIST;
use crate::services::transaction_processor::TOKEN_TABLE;
use crate::services::transaction_processor::TokenState;
use crate::services::transaction_processor::update_token_state;
use crate::transaction::pump_sell;
use anyhow::anyhow;
use anyhow::{Context, Result}; // 引入 `anyhow::Result`
use base64::{Engine, engine::general_purpose};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::Local;
use dotenvy::dotenv;
use futures_util::SinkExt;
use grpc_client::{AppError, TransactionFormat, YellowstoneGrpc};
use log::{error, info};
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::{fs, time};
use tokio_stream::StreamExt;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeRequestPing,
    subscribe_update::UpdateOneof,
};

#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: u64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub fee_recipient: Pubkey,
    pub fee_basis_points: u64,
    pub fee: u64,
    pub creator: Pubkey,
    pub creator_fee_basis_points: u64,
    pub creator_fee: u64,
}

const PROGRAM_DATA: &str = "Program data: ";
const SOL_DECIMALS: f64 = 1_000_000_000.0; // 10^9
const TOKEN_DECIMALS: f64 = 1_000_000.0; // 10^6
pub const BLACKLIST_PATH: &str = "blacklist.txt";

pub static GRPC_NORMAL: AtomicBool = AtomicBool::new(false);

pub trait EventTrait: Sized + std::fmt::Debug {
    fn discriminator() -> [u8; 8];
    fn from_bytes(bytes: &[u8]) -> Result<Self, AppError>;
    fn valid_discrminator(head: &[u8]) -> bool;

    fn parse_logs<T: EventTrait + Clone>(logs: &[String]) -> Option<T> {
        logs.iter().rev().find_map(|log| {
            let payload = log.strip_prefix(PROGRAM_DATA)?;
            let bytes = general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| AppError::from(anyhow!(e.to_string())))
                .ok()?;

            let (discr, rest) = bytes.split_at(8);
            if Self::valid_discrminator(discr) {
                T::from_bytes(rest).ok()
            } else {
                None
            }
        })
    }
}

pub async fn add_to_blacklist(address: &str) -> Result<(), std::io::Error> {
    use tokio::{fs, io::AsyncWriteExt};

    let mut bl = BLACKLIST.write().await;

    if !bl.contains(address) {
        bl.insert(address.to_string());

        let mut content = String::new();
        if let Ok(existing) = fs::read_to_string(BLACKLIST_PATH).await {
            content = existing;
        }

        if !content.contains(address) {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(BLACKLIST_PATH)
                .await?;

            // 确保前一行结尾是换行符
            if !content.ends_with('\n') {
                file.write_all(b"\n").await?;
            }

            file.write_all(format!("{}\n", address).as_bytes()).await?;
            file.flush().await?;
        }
    }

    Ok(())
}

pub async fn update_price_and_maybe_sell(mint: Pubkey, new_price: f64) {
    let mut entry = TOKEN_TABLE.entry(mint).or_insert(TokenState {
        first_buy_price: None,
        current_price: None,
        balance: None,
        bonding_curve: None,
        sell_stage: 0,
        highest_price: new_price,
        last_tx_time: None,
        last_tx_price: None,
        token_creator: Pubkey::default(),
        first_buy_time: None,
    });

    entry.current_price = Some(new_price);

    let creator_pubkey = entry.token_creator.clone();

    // 更新最高价
    if new_price > entry.highest_price {
        entry.highest_price = new_price;
    }

    let Some(first_buy_price) = entry.first_buy_price else {
        println!("[DEBUG] TokenState 中未记录首次买入价: {}", mint);
        return;
    };

    let Some(balance) = entry.balance else {
        println!("[DEBUG] 未记录余额，跳过: {}", mint);
        return;
    };

    let change = (new_price - first_buy_price) / first_buy_price;

    if entry.sell_stage == 0 {
        if let Some(buy_time) = entry.first_buy_time {
            let held_duration = buy_time.elapsed();
            if held_duration.as_millis() >= 2000 && change < 0.20 {
                let mint_clone = mint;
                println!(
                    "[⏱️快速止盈: {}] 持仓超过 2 秒，涨幅未达 20%（当前 {:.2}%），清仓 {} 个代币",
                    mint,
                    change * 100.0,
                    balance
                );

                tokio::spawn(async move {
                    pump_sell(mint_clone, creator_pubkey, balance).await;
                    TOKEN_TABLE.remove(&mint_clone);
                });
                return;
            }
        }
    }
    // let now = Instant::now();

    // // ---------------- 清仓逻辑 --------------------
    // // 启动计时
    // if entry.last_tx_time.is_none() {
    //     entry.last_tx_time = Some(now);
    //     entry.last_tx_price = Some(new_price);
    // } else if let (Some(last_time), Some(last_price)) = (entry.last_tx_time, entry.last_tx_price) {
    //     if now.duration_since(last_time) >= Duration::from_secs(6) {
    //         let price_change = (new_price - last_price).abs() / last_price;
    //         if price_change <= 0.10 {
    //             let mint_clone = mint;
    //             let balance = entry.balance.unwrap_or(0);
    //             println!(
    //                 "[清仓: {}, 无波动卖出] 6秒内价格变动 {:.2}%，清仓 {} 个代币",
    //                 mint,
    //                 price_change * 100.0,
    //                 balance
    //             );

    //             tokio::spawn(async move {
    //                 pump_sell(mint_clone, balance).await;
    //                 TOKEN_TABLE.remove(&mint_clone);
    //             });
    //             return;
    //         } else {
    //             // 价格波动大，重置清仓计时
    //             entry.last_tx_time = Some(now);
    //             entry.last_tx_price = Some(new_price);
    //         }
    //     }
    // }

    // ---------------- 其他卖出逻辑 --------------------

    // 已经至少卖过一轮，并且跌破最高价的 10%
    if entry.sell_stage >= 1 && new_price <= entry.highest_price * 0.95 {
        let mint_clone = mint;
        if balance > 0 {
            println!(
                "[😂止盈转止损: {}, 已达阶段{}] 价格从 {:.12} 跌至 {:.12}，跌幅超 5%，清仓 {} 个代币",
                mint, entry.sell_stage, entry.highest_price, new_price, balance
            );

            tokio::spawn(async move {
                pump_sell(mint_clone, creator_pubkey, balance).await;
                TOKEN_TABLE.remove(&mint_clone);
            });
            return;
        }
    }

    if entry.sell_stage == 0 && change >= 0.20 {
        let mint_clone = mint;
        let amount = (balance as f64 * 0.5).round() as u64;
        println!(
            "[出售代币🪙: {}, 阶段1] 涨幅达到 20%，卖出 50% {} 个代币, 变动: {:.2}%",
            mint,
            amount,
            change * 100.0
        );

        entry.balance = Some(balance - amount);
        entry.sell_stage = 1;

        tokio::spawn(async move {
            pump_sell(mint_clone, creator_pubkey, amount).await;
        });
    } else if entry.sell_stage == 1 && change >= 0.40 {
        let mint_clone = mint;
        let amount = (balance as f64 * 0.4).round() as u64;
        println!(
            "[出售代币🪙: {}, 阶段2] 涨幅达到 40%，卖出 剩余40% {} 个代币, 变动: {:.2}%",
            mint,
            amount,
            change * 100.0
        );

        entry.balance = Some(balance - amount);
        entry.sell_stage = 2;

        tokio::spawn(async move {
            pump_sell(mint_clone, creator_pubkey, amount).await;
        });
    } else if entry.sell_stage == 2 && change >= 0.60 {
        let mint_clone = mint;
        println!(
            "[出售代币🪙: {}, 阶段3] 涨幅达到 60%，全部卖出 {} 个代币, 变动: {:.2}%",
            mint,
            balance,
            change * 100.0
        );

        entry.sell_stage = 3;

        tokio::spawn(async move {
            pump_sell(mint_clone, creator_pubkey, balance).await;
            TOKEN_TABLE.remove(&mint_clone);
        });
    } else if entry.sell_stage < 3 && change <= -0.05 {
        let mint_clone = mint;
        println!(
            "[出售代币🪙: {}, 触发止损] 价格跌破成本价，全部卖出 {} 个代币, 变动: {:.2}%",
            mint,
            balance,
            change * 100.0
        );
        tokio::spawn(async move {
            pump_sell(mint_clone, creator_pubkey, balance).await;
            add_to_blacklist(&creator_pubkey.to_string()).await;

            TOKEN_TABLE.remove(&mint_clone);
        });
    } else {
        println!(
            "[😌代币地址: {}] 价格变动未超过阈值，首次买入价: {:.12}, 当前价: {:.12}, 变动: {:.2}%",
            mint,
            first_buy_price,
            new_price,
            change * 100.0
        );
    }
}

impl EventTrait for TradeEvent {
    fn discriminator() -> [u8; 8] {
        [189, 219, 127, 211, 78, 230, 97, 238]
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, AppError> {
        Self::try_from_slice(bytes).map_err(|e| AppError::from(anyhow!(e.to_string())))
    }

    fn valid_discrminator(discr: &[u8]) -> bool {
        discr == Self::discriminator()
    }
}

pub async fn run_yellowstone_listener() -> Result<(), AppError> {
    let url = std::env::var("YELLOWSTONE_GRPC_URL").expect("YELLOWSTONE_GRPC_URL must be set");
    let grpc = YellowstoneGrpc::new(url.clone(), None);
    let client = grpc.build_client().await?;

    let subscribe_request = SubscribeRequest {
        transactions: HashMap::from([(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include: vec![], // 全链监听
                account_exclude: vec![],
                account_required: vec![],
            },
        )]),
        commitment: Some(CommitmentLevel::Processed.into()),
        ..Default::default()
    };

    const RETRY_INTERVAL: Duration = Duration::from_secs(60); // 每10分钟一次

    loop {
        let (mut subscribe_tx, mut stream) = client
            .lock()
            .await
            .subscribe_with_request(Some(subscribe_request.clone()))
            .await?;

        println!("订阅 grpc 成功: [{}]", url);

        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => match msg.update_oneof {
                    Some(UpdateOneof::Transaction(sut)) => {
                        GRPC_NORMAL.store(true, Ordering::Relaxed);

                        let transaction: TransactionFormat = sut.into();

                        // 抽取本次交易的账户地址
                        let accounts_in_tx: Vec<String> = transaction
                            .account_keys
                            .iter()
                            .map(|k| k.to_string())
                            .collect();

                        // 从 TOKEN_TABLE 获取所有代币对应的 bonding_curve 地址
                        let matched_token_mint = TOKEN_TABLE.iter().find_map(|entry| {
                            let token_state = entry.value(); // 获取 TokenState
                            if let Some(bonding_curve) = &token_state.bonding_curve {
                                if accounts_in_tx.contains(&bonding_curve.to_string()) {
                                    return Some(entry.key().clone()); // 获取 mint
                                }
                            }
                            None
                        });

                        if let Some(mint_hit) = matched_token_mint {
                            println!("找到匹配的 mint: {:?}", mint_hit)
                        } else {
                            continue;
                        };

                        // 命中 bonding_curve，处理交易内容
                        if let Some(meta) = transaction.meta {
                            let logs = meta.log_messages.unwrap_or_default();
                            if logs.is_empty() {
                                println!("⚠️ log_messages 为空，跳过");
                                continue;
                            }

                            if let Some(trade_event) = TradeEvent::parse_logs::<TradeEvent>(&logs) {
                                let price_in_sol = (trade_event.virtual_sol_reserves as f64
                                    / SOL_DECIMALS)
                                    / (trade_event.virtual_token_reserves as f64 / TOKEN_DECIMALS);

                                update_price_and_maybe_sell(trade_event.mint, price_in_sol).await;
                                println!(
                                    "mint: {}, 更新价格: {:.12}",
                                    trade_event.mint, price_in_sol
                                );
                            } else {
                                println!("❌ TradeEvent::parse_logs 失败，跳过");
                            }
                        } else {
                            error!("meta not found");
                        }
                    }
                    Some(UpdateOneof::Ping(_)) => {
                        let _ = subscribe_tx
                            .send(SubscribeRequest {
                                ping: Some(SubscribeRequestPing { id: 1 }),
                                ..Default::default()
                            })
                            .await;
                        println!("service is ping: {:#?}", Local::now());
                    }
                    Some(UpdateOneof::Pong(_)) => {
                        println!("service is pong: {:#?}", Local::now());
                    }
                    None => {
                        error!("读取到空的 grpc 消息, 退出");
                        break;
                    }
                    _ => {}
                },
                Err(error) => {
                    error!("读取 grpc 消息失败, 退出: {error:?}");
                    break;
                }
            }
        }

        GRPC_NORMAL.store(false, Ordering::Relaxed);

        error!(
            "grpc 流已关闭, 等待 {} 秒后重试: [{}]",
            RETRY_INTERVAL.as_secs(),
            url
        );
        time::sleep(RETRY_INTERVAL).await;
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn test_yellowstone_listener() {
        dotenvy::dotenv().ok();
        // 配置tokio运行
        run_yellowstone_listener().await.unwrap();
    }
}
