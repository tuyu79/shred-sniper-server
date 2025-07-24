use crate::config::{
    BUY_ENABLED, JITO_FEE, MAX_SOL, WHITELIST_AVG, WHITELIST_AVG_USER, WHITELIST_COUNT,
    WHITELIST_ENABLED, WHITELIST_HOLD_LESS_5_SEC_COUNT, WHITELIST_MID, WHITELIST_MIN_HOLD,
    WHITELIST_PROFIT, WHITELIST_TOP_3_BUY, ZERO_SLOT_BUY_FEE, ZERO_SLOT_SELL_FEE,
};
use crate::monitor::{BLACKLIST_PATH, add_to_blacklist};
use crate::services::transaction_processor::BLACKLIST;
use serde_json::to_string;
use sniper_protos::shared::config_service_server::{ConfigService, ConfigServiceServer};
use sniper_protos::shared::{
    BlackListResponse, BlacklistRequest, CommonResponse, Config, EmptyRequest, WhitelistConfig,
};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use tonic::{Request, Response, Status};

const ENV_PATH: &str = ".env";

struct MyService {}

#[tonic::async_trait]
impl ConfigService for MyService {
    async fn get_config(&self, request: Request<EmptyRequest>) -> Result<Response<Config>, Status> {
        unsafe {
            Ok(Response::new(Config {
                buy_enabled: *BUY_ENABLED,
                max_sol: *MAX_SOL,
                whitelist_enabled: *WHITELIST_ENABLED,
                jito_fee: *JITO_FEE,
                zero_slot_buy_fee: *ZERO_SLOT_BUY_FEE,
                zero_slot_sell_fee: *ZERO_SLOT_SELL_FEE,
            }))
        }
    }

    async fn update_config(
        &self,
        request: Request<Config>,
    ) -> Result<Response<CommonResponse>, Status> {
        let config = request.into_inner();

        let mut map = read_env_file(ENV_PATH).await?;
        map.insert("BUY_ENABLED".to_string(), config.buy_enabled.to_string());
        map.insert("MAX_SOL".to_string(), config.max_sol.to_string());
        map.insert("WHITELIST_ENABLED".to_string(), config.whitelist_enabled.to_string());
        map.insert("JITO_FEE".to_string(), config.jito_fee.to_string());
        map.insert("ZERO_SLOT_BUY_FEE".to_string(), config.zero_slot_buy_fee.to_string());
        map.insert("ZERO_SLOT_SELL_FEE".to_string(), config.zero_slot_sell_fee.to_string());

        write_env_file(ENV_PATH, &map).await?;

        unsafe {
            *BUY_ENABLED = config.buy_enabled;
            *MAX_SOL = config.max_sol;
            *WHITELIST_ENABLED = config.whitelist_enabled;
            *JITO_FEE = config.jito_fee;
            *ZERO_SLOT_BUY_FEE = config.zero_slot_buy_fee;
            *ZERO_SLOT_SELL_FEE = config.zero_slot_sell_fee;
        }

        Ok(Response::new(CommonResponse {
            result: "ok".to_string(),
        }))
    }

    async fn get_whitelist_config(
        &self,
        request: Request<EmptyRequest>,
    ) -> Result<Response<WhitelistConfig>, Status> {
        unsafe {
            Ok(Response::new(WhitelistConfig {
                profit: *WHITELIST_PROFIT,
                avg: *WHITELIST_AVG,
                count: *WHITELIST_COUNT,
                mid: *WHITELIST_MID,
                hold_less_5_sec_count: *WHITELIST_HOLD_LESS_5_SEC_COUNT,
                min_hold: *WHITELIST_MIN_HOLD,
                avg_user: *WHITELIST_AVG_USER,
                top_3_buy: *WHITELIST_TOP_3_BUY,
            }))
        }
    }

    async fn update_whitelist_config(
        &self,
        request: Request<WhitelistConfig>,
    ) -> Result<Response<CommonResponse>, Status> {
        let config = request.into_inner();

        let mut map = read_env_file(ENV_PATH).await?;
        map.insert("WHITELIST_PROFIT".to_string(), config.profit.to_string());
        map.insert("WHITELIST_AVG".to_string(), config.avg.to_string());
        map.insert("WHITELIST_COUNT".to_string(), config.count.to_string());
        map.insert("WHITELIST_MID".to_string(), config.mid.to_string());
        map.insert("WHITELIST_HOLD_LESS_5_SEC_COUNT".to_string(), config.hold_less_5_sec_count.to_string());
        map.insert("WHITELIST_MIN_HOLD".to_string(), config.min_hold.to_string());
        map.insert("WHITELIST_AVG_USER".to_string(), config.avg_user.to_string());
        map.insert("WHITELIST_TOP_3_BUY".to_string(), config.top_3_buy.to_string());

        write_env_file(ENV_PATH, &map).await?;

        unsafe {
            *WHITELIST_PROFIT = config.profit;
            *WHITELIST_AVG = config.avg;
            *WHITELIST_COUNT = config.count;
            *WHITELIST_MID = config.mid;
            *WHITELIST_HOLD_LESS_5_SEC_COUNT = config.hold_less_5_sec_count;
            *WHITELIST_MIN_HOLD = config.min_hold;
            *WHITELIST_AVG_USER = config.avg_user;
            *WHITELIST_TOP_3_BUY = config.top_3_buy;
        }

        Ok(Response::new(CommonResponse {
            result: "ok".to_string(),
        }))
    }

    async fn get_blacklist(
        &self,
        request: Request<EmptyRequest>,
    ) -> Result<Response<BlackListResponse>, Status> {
        let blacklist = BLACKLIST.read().await;
        Ok(Response::new(BlackListResponse {
            items: blacklist.iter().cloned().collect(),
        }))
    }

    async fn add_blacklist(
        &self,
        request: Request<BlacklistRequest>,
    ) -> Result<Response<CommonResponse>, Status> {
        add_to_blacklist(&request.into_inner().item).await?;

        Ok(Response::new(CommonResponse {
            result: "ok".to_string(),
        }))
    }

    async fn remove_blacklist(
        &self,
        request: Request<BlacklistRequest>,
    ) -> Result<Response<CommonResponse>, Status> {
        let request = request.into_inner();
        let mut blacklist_set = read_blacklist(BLACKLIST_PATH).await?;

        blacklist_set.remove(&request.item);

        write_blacklist(BLACKLIST_PATH, &blacklist_set).await?;

        let mut blacklist = BLACKLIST.write().await;

        blacklist.remove(&request.item);

        Ok(Response::new(CommonResponse {
            result: "ok".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_blacklist() {
        let mut set = read_blacklist(BLACKLIST_PATH).await.unwrap();
        set.remove("456");
        write_blacklist(BLACKLIST_PATH, &set).await.unwrap();

        println!("blacklist set : {:?}", set);
    }
}

async fn read_blacklist(path: &str) -> Result<HashSet<String>, std::io::Error> {
    let content = fs::read_to_string(path).await?;
    let mut blacklist_set = HashSet::new();

    for line in content.lines() {
        // 忽略注释和空行
        let line = line.trim();
        blacklist_set.insert(line.to_string());
    }

    Ok(blacklist_set)
}

async fn write_blacklist(
    path: &str,
    blacklist_set: &HashSet<String>,
) -> Result<(), std::io::Error> {
    let mut content = String::new();

    // 写入所有键值对
    for key in blacklist_set {
        content.push_str(&format!("{}\n", key));
    }

    fs::write(path, content).await?;
    Ok(())
}

/// 读取 .env 文件并解析为键值对 HashMap
async fn read_env_file(path: &str) -> Result<HashMap<String, String>, std::io::Error> {
    let content = fs::read_to_string(path).await?;
    let mut env_map = HashMap::new();

    for line in content.lines() {
        // 忽略注释和空行
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        // 分割键值对（支持 = 或 : 分隔）
        if let Some((key, value)) = line.split_once('=') {
            env_map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    Ok(env_map)
}

/// 将键值对写回 .env 文件
async fn write_env_file(
    path: &str,
    env_map: &HashMap<String, String>,
) -> Result<(), std::io::Error> {
    let mut content = String::new();

    // 写入所有键值对
    for (key, value) in env_map {
        content.push_str(&format!("{}={}\n", key, value));
    }

    fs::write(path, content).await?;
    Ok(())
}

/// 修改 .env 文件中指定键的值（不存在则新增）
async fn update_env_var(env_path: &str, key: &str, value: &str) -> Result<(), std::io::Error> {
    // 读取现有 .env 内容
    let mut env_map = if Path::exists(Path::new(env_path)) {
        read_env_file(env_path).await?
    } else {
        HashMap::new() // 文件不存在则创建新的
    };

    // 修改或新增键值对
    env_map.insert(key.to_string(), value.to_string());

    // 写回文件
    write_env_file(env_path, &env_map).await?;
    Ok(())
}

pub async fn start_server_thread() {
    let exit = AtomicBool::new(false);

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.spawn(async move {
            tonic::transport::Server::builder()
                .add_service(ConfigServiceServer::new(MyService {}))
                .serve(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 9090))
                .await
                .unwrap();
        });

        while !exit.load(Ordering::Relaxed) {}
    });
}
