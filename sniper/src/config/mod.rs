use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use std::io::{Error, ErrorKind};
use std::{env, fs};

// 定义要查找的程序ID (Base58格式)
pub const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub static mut BUY_ENABLED: Lazy<bool> = Lazy::new(|| env::var("BUY_ENABLED").expect("没有设置 BUY_ENABLED").parse().unwrap());
pub static NONCE_PUBKEY: Lazy<String> = Lazy::new(|| env::var("NONCE_PUBKEY").expect("没有设置 NONCE_PUBKEY"));
pub static PRIVATE_KEY: Lazy<String> = Lazy::new(|| env::var("PRIVATE_KEY").expect("没有设置 PRIVATE_KEY"));
pub static PUBLIC_KEY: Lazy<String> = Lazy::new(|| env::var("PUBLIC_KEY").expect("没有设置 PUBLIC_KEY"));
pub static JITO_RPC_ENDPOINTS: Lazy<String> = Lazy::new(|| env::var("JITO_RPC_ENDPOINTS").expect("没有设置 JITO_RPC_ENDPOINTS"));
pub static ZERO_SLOT_RPC_ENDPOINTS: Lazy<String> = Lazy::new(|| env::var("ZERO_SLOT_RPC_ENDPOINTS").expect("没有设置 ZERO_SLOT_RPC_ENDPOINTS"));
pub static JITO_SHRED_URL: Lazy<String> = Lazy::new(|| env::var("JITO_SHRED_URL").expect("没有设置 JITO_SHRED_URL"));
pub static mut MAX_SOL: Lazy<f64> = Lazy::new(|| env::var("MAX_SOL").expect("没有设置 MAX_SOL").parse().unwrap());
pub static mut JITO_FEE: Lazy<f64> = Lazy::new(|| env::var("JITO_FEE").expect("没有设置 JITO_FEE").parse().unwrap());
pub static mut ZERO_SLOT_BUY_FEE: Lazy<f64> = Lazy::new(|| env::var("ZERO_SLOT_BUY_FEE").expect("没有设置 ZERO_SLOT_BUY_FEE").parse().unwrap());
pub static mut ZERO_SLOT_SELL_FEE: Lazy<f64> = Lazy::new(|| env::var("ZERO_SLOT_SELL_FEE").expect("没有设置 ZERO_SLOT_SELL_FEE").parse().unwrap());
pub static mut WHITELIST_ENABLED: Lazy<bool> = Lazy::new(|| env::var("WHITELIST_ENABLED").expect("没有设置 WHITELIST_ENABLED").parse().unwrap());

// 白名单查询配置
pub static mut WHITELIST_PROFIT: Lazy<f64> = Lazy::new(|| env::var("WHITELIST_PROFIT").expect("没有设置 WHITELIST_PROFIT").parse().unwrap());
pub static mut WHITELIST_AVG: Lazy<i64> = Lazy::new(|| env::var("WHITELIST_AVG").expect("没有设置 WHITELIST_AVG").parse().unwrap());
pub static mut WHITELIST_COUNT: Lazy<i64> = Lazy::new(|| env::var("WHITELIST_COUNT").expect("没有设置 WHITELIST_COUNT").parse().unwrap());
pub static mut WHITELIST_MID: Lazy<i64> = Lazy::new(|| env::var("WHITELIST_MID").expect("没有设置 WHITELIST_MID").parse().unwrap());
pub static mut WHITELIST_HOLD_LESS_5_SEC_COUNT: Lazy<i64> = Lazy::new(|| env::var("WHITELIST_HOLD_LESS_5_SEC_COUNT").expect("没有设置 WHITELIST_HOLD_LESS_5_SEC_COUNT").parse().unwrap());
pub static mut WHITELIST_MIN_HOLD: Lazy<i64> = Lazy::new(|| env::var("WHITELIST_MIN_HOLD").expect("没有设置 WHITELIST_MIN_HOLD").parse().unwrap());
pub static mut WHITELIST_AVG_USER: Lazy<i64> = Lazy::new(|| env::var("WHITELIST_AVG_USER").expect("没有设置 WHITELIST_AVG_USER").parse().unwrap());
pub static mut WHITELIST_TOP_3_BUY: Lazy<f64> = Lazy::new(|| env::var("WHITELIST_TOP_3_BUY").expect("没有设置 WHITELIST_TOP_3_BUY").parse().unwrap());
