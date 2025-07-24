mod server;

use crate::server::start_server_thread;
use anyhow::Result;
use anyhow::anyhow;
use base64::{Engine, engine::general_purpose};
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::Local;
use chrono::Utc;
use dotenvy::dotenv;
use futures_util::SinkExt;
use futures_util::future::ok;
use grpc_client::{AppError, TransactionFormat, YellowstoneGrpc};
use log::warn;
use log::{error, info};
use solana_sdk::pubkey::Pubkey;
use sqlx::FromRow;
use sqlx::PgPool;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use tokio_stream::StreamExt;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeRequestPing,
    subscribe_update::UpdateOneof,
};

type TokenMap = Arc<RwLock<HashMap<Pubkey, TokenState>>>;

#[derive(Debug, Clone, FromRow)]
pub struct TokenState {
    pub token_creator: Pubkey,
    pub token_address: Pubkey,
    pub dev_initial_buy: Option<u64>,        // 计算出来
    pub dev_profit: Option<f64>,             // 计算出来
    pub dev_total_sell: u64,                 // 新增字段：累计卖出SOL
    pub dev_current_holding: u64,            // dev 当前持有的 token 数量
    pub dev_holding_start_time: Option<i64>, // 创建时记录
    pub dev_holding_duration: Option<i64>,   // 卖出时计算出来
}

#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
    pub creator: Pubkey,
    pub timestamp: i64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub token_total_supply: u64,
}

#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
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

impl EventTrait for CreateEvent {
    fn discriminator() -> [u8; 8] {
        [27, 114, 169, 77, 222, 235, 99, 118]
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, AppError> {
        Self::try_from_slice(bytes).map_err(|e| AppError::from(anyhow!(e.to_string())))
    }

    fn valid_discrminator(discr: &[u8]) -> bool {
        discr == Self::discriminator()
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

pub async fn insert_token_state(
    pool: &PgPool,
    token_state: &TokenState,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO token_states (
            token_creator,
            token_address,
            dev_initial_buy,
            dev_profit,
            dev_holding_start_time,
            dev_holding_duration
        ) VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        token_state.token_creator.to_string(),
        token_state.token_address.to_string(),
        token_state.dev_initial_buy.map(|v| v as i64),
        token_state.dev_profit,
        token_state.dev_holding_start_time.unwrap_or(0),
        token_state.dev_holding_duration.unwrap_or(0),
    )
    .execute(pool)
    .await?;

    Ok(())
}

const SQL: &str = r#"
DROP TABLE IF EXISTS filtered_devs;

CREATE TABLE filtered_devs AS
WITH token_stats AS (
    SELECT
        token_creator,
        COUNT(*) AS token_count,
        AVG(dev_holding_duration) AS avg_holding_seconds,
        SUM(dev_profit) AS total_profit_sol,
        SUM(CASE WHEN dev_holding_duration > 5 AND dev_holding_duration < 10 THEN 1 ELSE 0 END) AS mid_hold_count
    FROM
        token_states
    WHERE
        dev_profit IS NOT NULL
    GROUP BY
        token_creator
)
SELECT *
FROM token_stats
WHERE
    avg_holding_seconds > 10
    AND total_profit_sol > 0.1
    AND token_count > 5
    AND mid_hold_count <= 2
ORDER BY total_profit_sol DESC;
"#;

async fn refresh_filtered_devs(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(SQL).execute(pool).await?;
    Ok(())
}

async fn process_subscription(
    pool: &PgPool,
    token_map: TokenMap,
    url: &str,
    addrs: Vec<String>,
) -> Result<(), AppError> {
    let grpc = YellowstoneGrpc::new(url.to_string(), None);
    let client = grpc.build_client().await?;

    let subscribe_request = SubscribeRequest {
        transactions: HashMap::from([(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include: addrs.clone(),
                account_exclude: vec![],
                account_required: vec![],
            },
        )]),
        commitment: Some(CommitmentLevel::Processed.into()),
        ..Default::default()
    };

    let (mut subscribe_tx, mut stream) = client
        .lock()
        .await
        .subscribe_with_request(Some(subscribe_request))
        .await?;

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                match msg.update_oneof {
                    Some(UpdateOneof::Transaction(sut)) => {
                        let transaction: TransactionFormat = sut.into();
                        let meta = match transaction.meta {
                            Some(meta) => meta,
                            None => {
                                error!("meta not found");
                                continue;
                            }
                        };

                        let logs = &meta.log_messages.unwrap_or_default();

                        if let Some(create_event) = CreateEvent::parse_logs::<CreateEvent>(logs) {
                            info!("create_event: {:?}", create_event);
                            token_map.write().await.insert(
                                create_event.mint,
                                TokenState {
                                    token_creator: create_event.creator,
                                    token_address: create_event.mint,
                                    dev_initial_buy: Some(0),
                                    dev_current_holding: 0,
                                    dev_profit: None,
                                    dev_total_sell: 0,
                                    dev_holding_start_time: Some(create_event.timestamp),
                                    dev_holding_duration: None,
                                },
                            );
                        }

                        if let Some(trade_event) = TradeEvent::parse_logs::<TradeEvent>(logs) {
                            let mut token_map = token_map.write().await;
                            if let Some(token_state) = token_map.get_mut(&trade_event.mint) {
                                if trade_event.user == token_state.token_creator {
                                    if trade_event.is_buy {
                                        token_state.dev_current_holding += trade_event.token_amount;
                                        token_state.dev_initial_buy = Some(
                                            token_state.dev_initial_buy.unwrap_or(0)
                                                + trade_event.sol_amount,
                                        );
                                        token_state
                                            .dev_holding_start_time
                                            .get_or_insert(trade_event.timestamp);
                                    } else {
                                        if token_state.dev_current_holding
                                            >= trade_event.token_amount
                                        {
                                            token_state.dev_current_holding -=
                                                trade_event.token_amount;
                                            token_state.dev_total_sell += trade_event.sol_amount;

                                            if token_state.dev_current_holding == 0 {
                                                let profit = token_state.dev_total_sell as i128
                                                    - token_state.dev_initial_buy.unwrap_or(0)
                                                        as i128;
                                                let profit_sol = profit as f64 / SOL_DECIMALS;
                                                let duration = trade_event.timestamp
                                                    - token_state
                                                        .dev_holding_start_time
                                                        .unwrap_or(trade_event.timestamp);

                                                token_state.dev_profit = Some(profit_sol);
                                                token_state.dev_holding_duration = Some(duration);

                                                insert_token_state(pool, token_state).await?;
                                            }
                                        } else {
                                            warn!("Dev is selling more than current holding?");
                                        }
                                    }
                                }

                                // 过滤掉 dev 自己行为
                                if trade_event.user != token_state.token_creator {
                                    sqlx::query!(
                                        r#"
                                            INSERT INTO token_trades (
                                                token_address, useraddr, is_buy, sol_amount, token_amount, timestamp
                                            ) VALUES ($1, $2, $3, $4, $5, $6)
                                        "#,
                                        trade_event.mint.to_string(),
                                        trade_event.user.to_string(),
                                        trade_event.is_buy,
                                        trade_event.sol_amount as i64,
                                        trade_event.token_amount as i64,
                                        trade_event.timestamp
                                    )
                                        .execute(pool)
                                        .await?;
                                }

                                info!("TradeEvent updated: {:?}", token_state);
                            }
                        }
                    }
                    Some(UpdateOneof::Ping(_)) => {
                        let _ = subscribe_tx
                            .send(SubscribeRequest {
                                ping: Some(SubscribeRequestPing { id: 1 }),
                                ..Default::default()
                            })
                            .await;
                        info!("service ping: {}", Local::now());
                    }
                    Some(UpdateOneof::Pong(_)) => {
                        info!("service pong: {}", Local::now());
                    }
                    _ => {}
                }
            }
            Err(err) => {
                error!("订阅流错误: {:?}", err);
                return Err(AppError::from(anyhow!("Stream error: {:?}", err)));
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();

    pretty_env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = Arc::new(PgPool::connect(&database_url).await?);
    println!("✅ Connected to PostgreSQL");

    start_server_thread(pool.clone()).await;

    let token_map: TokenMap = Arc::new(RwLock::new(HashMap::new()));
    let url = std::env::var("YELLOWSTONE_GRPC_URL").expect("YELLOWSTONE_GRPC_URL must be set");
    let addrs = vec!["6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_string()];

    loop {
        let result = process_subscription(&pool, token_map.clone(), &url, addrs.clone()).await;

        if let Err(e) = result {
            error!("处理订阅失败: {:?}, 10 秒后重试...", e);
        } else {
            warn!("订阅正常结束（意外），10 秒后重试...");
        }

        sleep(Duration::from_secs(10)).await;
    }
}
