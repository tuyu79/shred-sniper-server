use bincode;
use jito_protos::shredstream::{
    SubscribeEntriesRequest, shredstream_proxy_client::ShredstreamProxyClient,
};
use std::io;
use std::time::Instant;
use tokio::runtime::Runtime;
use tokio::time::sleep;

use crate::services::transaction_processor::{
    TransactionProcessor, watch_blacklist_txt, watch_whitelist_txt,
};

use crate::api::get_rpc_client;
use crate::api::{APP_STATE, AppState};
use crate::monitor::run_yellowstone_listener;
use crate::tx::{keep_alive_loop, start_blockhash_fetcher};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

use crate::config::{JITO_SHRED_URL, NONCE_PUBKEY, PRIVATE_KEY, PUBLIC_KEY, WHITELIST_AVG, WHITELIST_AVG_USER, WHITELIST_COUNT, WHITELIST_HOLD_LESS_5_SEC_COUNT, WHITELIST_MID, WHITELIST_MIN_HOLD, WHITELIST_PROFIT, WHITELIST_TOP_3_BUY};
use crate::server::start_server_thread;
use analyzer_protos::shared::WhitelistRequest;
use analyzer_protos::shared::whitelist_service_client::WhitelistServiceClient;
use futures::executor::block_on;
use futures_util::StreamExt;
use log::error;
use reqwest::{Client, Request};
use serde::Deserialize;
use serde_json::json;
use solana_account_decoder::{UiAccountData, UiAccountEncoding};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcTokenAccountsFilter};
use solana_client::rpc_request::{RpcRequest, TokenAccountsFilter};
use solana_client::rpc_response::{RpcKeyedAccount, RpcResult};
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::program_pack::Pack;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::signer::keypair;
use solana_sdk::transaction::Transaction;
use spl_token::state::Account;
use spl_token_2022::instruction::close_account;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

#[derive(Deserialize)]
struct Creator {
    token_creator: String,
}

pub struct JitoClient;

async fn fetch_data_from_api(
    profit: f64,
    avg: i32,
    count: i32,
    mid: i32,
    hold_less_5_sec_count: i32,
    minhold: i32,
    avguser: i32,
    top3buy: f64,
) -> Result<Vec<Creator>, reqwest::Error> {
    // 创建 URL 并添加查询参数
    let url = "http://127.0.0.1:8090/query";
    let client = Client::new();

    // 发送 GET 请求，带上查询参数
    let res = client
        .get(url)
        .query(&[
            ("profit", &profit.to_string()),
            ("avg", &avg.to_string()),
            ("count", &count.to_string()),
            ("mid", &mid.to_string()),
            ("hold_less_5_sec_count", &hold_less_5_sec_count.to_string()),
            ("minhold", &minhold.to_string()),
            ("avguser", &avguser.to_string()),
            ("top3buy", &top3buy.to_string()),
        ])
        .send()
        .await?;

    // 解析返回的 JSON 数据
    let data: serde_json::Value = res.json().await?;

    // 提取 token_creator 字段并构建 Creator 对象
    let creators = data["data"]
        .as_array()
        .unwrap_or(&Vec::new()) // 如果没有数据，不会崩溃
        .iter()
        .filter_map(|item| {
            item.get("token_creator").and_then(|creator| {
                creator.as_str().map(|s| Creator {
                    token_creator: s.to_string(),
                })
            })
        })
        .collect::<Vec<_>>();

    Ok(creators)
}

async fn export_to_whitelist(creators: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // 打开文件进行写入
    let file = File::create("whitelist.txt")?;
    let mut writer = BufWriter::new(file);

    let len = creators.len();
    // 将所有 token_creator 写入文件
    for creator in creators {
        writeln!(writer, "{}", creator)?;
    }

    println!("✅ 成功写入 {} 个地址到 whitelist.txt", len);
    Ok(())
}

// 定时任务：每10分钟拉取一次数据并写入 whitelist.txt
async fn start_periodic_task() {
    let Ok(mut analyzer_grpc_client) =
        WhitelistServiceClient::connect("http://127.0.0.1:8090").await
    else {
        error!("连接 analyzer grpc 失败");
        return;
    };

    println!("连接 analyzer grpc 成功");

    const INTERVAL: Duration = Duration::from_secs(60); // 每60秒一次
    loop {
        time::sleep(INTERVAL).await;

        unsafe {
            let request = tonic::Request::new(WhitelistRequest {
                profit: *WHITELIST_PROFIT,
                avg: *WHITELIST_AVG,
                count: *WHITELIST_COUNT,
                mid: *WHITELIST_MID,
                hold_less_5_sec_count: *WHITELIST_HOLD_LESS_5_SEC_COUNT,
                min_hold: *WHITELIST_MIN_HOLD,
                avg_user: *WHITELIST_AVG_USER,
                top_3_buy: *WHITELIST_TOP_3_BUY,
            });

            let response = analyzer_grpc_client.get_whitelist(request).await;

            if response.is_err() {
                error!("获取白名单数据失败");
                continue;
            }

            let response = response.unwrap().into_inner();
            if let Err(e) = export_to_whitelist(response.filtered_creators).await {
                eprintln!("写入文件时出错: {}", e);
            }
        }
    }
}

async fn clean_token_account_task(client: Arc<RpcClient>) {
    const INTERVAL: Duration = Duration::from_secs(60); // 每10分钟一次
    loop {
        time::sleep(INTERVAL).await;

        let keypair = Keypair::from_base58_string(PRIVATE_KEY.as_str());

        let config = RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig {
                commitment: CommitmentLevel::Finalized,
            }),
            data_slice: None,
            min_context_slot: None,
        };

        let token_accounts: RpcResult<Vec<RpcKeyedAccount>> = client
            .send(
                RpcRequest::GetTokenAccountsByOwner,
                json!([
                    keypair.pubkey().to_string(),
                    RpcTokenAccountsFilter::ProgramId(spl_token::id().to_string()),
                    config
                ]),
            )
            .await;

        let token_accounts = token_accounts
            .map_err(|e| {
                println!("[‼️ERROR] 查询 token 账户失败, {:?}", e);
                e
            })
            .unwrap()
            .value;

        let token_accounts = token_accounts
            .into_iter()
            .filter(|x| {
                // 反序列化账户数据
                let token_account = Account::unpack(&x.account.data.decode().unwrap()).unwrap();
                token_account.amount == 0
            })
            .collect::<Vec<_>>();

        if token_accounts.is_empty() {
            println!("没有账户需要关闭, 退出");
            continue;
        }

        let chunks = token_accounts.chunks(10);
        for chunk in chunks {
            let close_ixs = chunk
                .into_iter()
                .map(|x| {
                    let account_pubkey = Pubkey::from_str(x.pubkey.as_str()).expect("Pubkey error");
                    spl_token::instruction::close_account(
                        &spl_token::id(),
                        &account_pubkey,
                        &keypair.pubkey(),
                        &keypair.pubkey(),
                        &[&keypair.pubkey()],
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();

            let blockhash = client.get_latest_blockhash().await.unwrap();
            let tx = Transaction::new_signed_with_payer(
                &close_ixs,
                Some(&keypair.pubkey()),
                &[&keypair],
                blockhash,
            );

            let result = client.send_and_confirm_transaction(&tx).await;
            match result {
                Ok(signature) => {
                    println!("Token 账户关闭成功，签名: {}", signature);
                }
                Err(_) => {
                    println!("[⚠️WARN] Token 账户关闭失败. 手动检查下. 等待下次重试");
                    break;
                }
            }
        }
    }
}

impl JitoClient {
    // 连接到Jito服务器并开始处理数据流
    async fn connect_and_process(jito_url: String) -> Result<(), io::Error> {
        const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);
        const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);

        // 初始化 AppState（包含 RPC 客户端）
        let app_state = Arc::new(AppState {
            client: get_rpc_client().expect("初始化 RPC 失败"),
        });

        // 全局设置 APP_STATE（只允许 set 一次）
        APP_STATE
            .set(app_state.clone())
            .expect("初始化 APP_STATE 失败");

        let app_state = APP_STATE.get().expect("AppState not initialized");
        let client = &app_state.client;

        // 定时任务，间隔10分钟执行一次
        tokio::spawn(clean_token_account_task(client.clone()));
        tokio::spawn(start_server_thread());
        tokio::spawn(start_periodic_task());
        tokio::spawn(watch_blacklist_txt("blacklist.txt"));
        tokio::spawn(watch_whitelist_txt("whitelist.txt"));

        let nonce_pubkey2 = Pubkey::from_str(NONCE_PUBKEY.as_str()).unwrap();
        let nonce_pubkey = Arc::new(nonce_pubkey2);
        start_blockhash_fetcher(&app_state, nonce_pubkey).await;

        tokio::spawn(keep_alive_loop());
        tokio::spawn(run_yellowstone_listener());

        let mut retry_delay = INITIAL_RETRY_DELAY;

        loop {
            println!("连接到Jito服务器 {}...", jito_url);

            // 创建client连接
            let client_result = ShredstreamProxyClient::connect(jito_url.clone()).await;

            match client_result {
                Ok(mut client) => {
                    println!("成功连接到Jito服务器！");
                    retry_delay = INITIAL_RETRY_DELAY; // 连接成功后重置重试间隔

                    let stream_result = client
                        .subscribe_entries(SubscribeEntriesRequest {})
                        .await
                        .map(|response| response.into_inner());

                    match stream_result {
                        Ok(mut stream) => {
                            println!("成功订阅Entry流！");

                            // 处理接收到的消息
                            loop {
                                match stream.message().await {
                                    Ok(Some(slot_entry)) => {
                                        let start_time = Instant::now();

                                        // 反序列化Entry
                                        let entries = match bincode::deserialize::<
                                            Vec<solana_entry::entry::Entry>,
                                        >(
                                            &slot_entry.entries
                                        ) {
                                            Ok(e) => e,
                                            Err(e) => {
                                                eprintln!("反序列化失败: {e}");
                                                continue;
                                            }
                                        };

                                        // 处理该slot中的所有交易
                                        let results = TransactionProcessor::process_entries(
                                            &entries,
                                            slot_entry.slot,
                                        )
                                        .await;

                                        // 只在有交易结果时才打印信息
                                        if results.has_results() {
                                            let _processing_time = start_time.elapsed();
                                            TransactionProcessor::print_results(&results);
                                        }
                                    }
                                    Ok(None) => {
                                        println!("Entry流结束，尝试重新连接...");
                                        break; // 流结束，跳出内部循环尝试重连
                                    }
                                    Err(e) => {
                                        eprintln!("读取Entry流错误: {}", e);
                                        break; // 出错，跳出内部循环尝试重连
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("订阅Entry流失败: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("连接Jito服务器失败: {}", e);
                }
            }

            // 等待一段时间后重试
            eprintln!("{}秒后重试连接...", retry_delay.as_secs());
            sleep(retry_delay).await;

            // 指数退避策略，但限制最大重试间隔
            retry_delay = std::cmp::min(retry_delay * 2, MAX_RETRY_DELAY);
        }
    }

    // 创建一个同步方法启动客户端
    pub fn start() -> Result<(), io::Error> {
        // 配置tokio运行
        let rt = Runtime::new().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // 启动处理循环（现在永远不会返回，除非发生致命错误）
        rt.block_on(Self::connect_and_process(JITO_SHRED_URL.to_string()))
    }
}
