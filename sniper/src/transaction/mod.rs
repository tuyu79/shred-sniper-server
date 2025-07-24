use crate::api::APP_STATE;
use crate::config::{MAX_SOL, NONCE_PUBKEY, PRIVATE_KEY};
use crate::tx::{tx_pump_buy, tx_pump_sell, update_nonce};
use anyhow::{anyhow, Error, Result}; // 引入 anyhow
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::read_keypair_file;
use solana_sdk::signature::Signer; // 导入 Signer trait
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    instruction::{AccountMeta, Instruction},
    rent::sysvar,
    system_instruction::create_account_with_seed,
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction::close_account;
use spl_token::instruction::initialize_account;
use std::collections::HashSet;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tonic::Status;
use tokio::{fs, time};

pub const GLOBAL_ACCOUNT: Pubkey =
    solana_sdk::pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");

pub const FEE_RECIPIENT: Pubkey =
    solana_sdk::pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");
const EVENT_AUTHORITY: Pubkey = solana_sdk::pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const PUMP_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

const PROXY_PROGRAM: Pubkey = solana_sdk::pubkey!("7uVmFk3SYJEgvD9unVPKzS19gSAg5b6CYzMP4er1HeKQ");

const RAYDIUM_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
const AMM_AUTHORITY: Pubkey = solana_sdk::pubkey!("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
const WSOL: Pubkey = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");

pub const PUMP_SELECTOR: &[u8; 8] = &[82, 225, 119, 231, 78, 29, 45, 70];
pub const PUMP_AMM_SELECTOR: &[u8; 8] = &[129, 59, 179, 195, 110, 135, 61, 2];
pub const PUMP_SELL_SELECTOR: &[u8; 8] = &[83, 225, 119, 231, 78, 29, 45, 70];
pub const PUMP_AMM_SELL_SELECTOR: &[u8; 8] = &[130, 59, 179, 195, 110, 135, 61, 2];
pub const EXPIRED_SLOT_SELECTOR: &[u8; 8] = &[169, 134, 33, 62, 168, 2, 246, 176];
pub const PUMPFUN_SELL_SELECTOR: &[u8; 8] = &[103, 6, 61, 18, 1, 218, 235, 234];

pub const ATA_SELECTOR: &[u8; 8] = &[22, 51, 53, 97, 247, 184, 54, 78];
pub const RAYDIUM_BUY_SELECTOR: &[u8; 8] = &[182, 77, 232, 39, 117, 138, 183, 72];
pub const RAYDIUM_SELL_SELECTOR: &[u8; 8] = &[183, 77, 232, 39, 117, 138, 183, 72];

const BONDING_CURVE_SEED: &[u8] = b"bonding-curve";

fn create_ata_token_account_instr(
    token_program: Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Instruction {
    let associated_token_account_idempotent =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            owner,
            owner,
            mint,
            &token_program,
        );
    associated_token_account_idempotent
}

pub async fn pump_buy(
    token_mint: Pubkey,
    bonding_curve: Pubkey,
    assoc_bonding_curve: Pubkey,
    creator_account: Pubkey,
    create_slot: u64,
    price: f64,
    token_amount: u64,
) -> Result<(), Error> {
    let start_build = Instant::now();
    let (creator_vault, _) = Pubkey::find_program_address(
        &[b"creator-vault", creator_account.as_ref()],
        &PUMP_PROGRAM_ID,
    );
    // println!("开始狙击代币: {}", token_mint);
    let signer = solana_sdk::signature::Keypair::from_base58_string(PRIVATE_KEY.as_str());

    let mut max_sol_cost  = 0 ;
    unsafe {
        max_sol_cost = (*MAX_SOL * 1_000_000_000.0) as u64; // lamports 0.15 sol
    }
    let token_price: f64 = price; // 单位是 SOL/个

    // println!("投入: {} SOL", max_sol);
    // println!("实际价格: {} SOL/token", token_price);
    // println!("尝试购买: {} 代币(含精度)", token_amount);

    // 指令1：过期 slot 检查
    let expiry_slot: u64 = create_slot + 1;
    let min_balance: u64 = 0; // 0.982 SOL

    // 构建过期检查的数据
    let mut expired_data = Vec::with_capacity(32); // 增加容量，以适应 padding
    expired_data.extend_from_slice(EXPIRED_SLOT_SELECTOR); // 添加选择器
    expired_data.extend_from_slice(&expiry_slot.to_le_bytes()); // 添加过期slot
    expired_data.extend_from_slice(&min_balance.to_le_bytes()); // 添加最低余额

    // 添加 16 字节的 padding 数据（可以是固定值或随机值）
    expired_data.extend_from_slice(&[0u8; 16]); // 或者你也可以随机生成这部分数据

    // 创建过期检查指令
    let expired_instruction = Instruction::new_with_bytes(
        PROXY_PROGRAM,
        &expired_data,
        vec![
            // 传入多个账户（可以是混淆账户）
            AccountMeta::new_readonly(assoc_bonding_curve, false),
            AccountMeta::new_readonly(bonding_curve, false),
            AccountMeta::new_readonly(creator_account, false),
        ],
    );

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(PUMP_SELECTOR); // 添加 pump 选择器
    data.extend_from_slice(&token_amount.to_le_bytes()); // 添加 token 数量
    data.extend_from_slice(&max_sol_cost.to_le_bytes()); // 添加最大 SOL 费用

    let token_mint = token_mint;

    let bonding_curve_address = bonding_curve;

    let associated_user = get_associated_token_address(&signer.pubkey(), &token_mint);

    let associated_bonding_curve = assoc_bonding_curve;

    let create_ata = create_ata_token_account_instr(spl_token::id(), &token_mint, &signer.pubkey());

    let pump_instruction = Instruction::new_with_bytes(
        PROXY_PROGRAM,
        &data,
        vec![
            AccountMeta::new_readonly(GLOBAL_ACCOUNT, false),
            AccountMeta::new(FEE_RECIPIENT, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(bonding_curve_address, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(associated_user, false),
            AccountMeta::new(signer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(creator_vault, false),
            AccountMeta::new_readonly(EVENT_AUTHORITY, false),
            AccountMeta::new_readonly(PUMP_PROGRAM_ID, false),
        ],
    );

    let instructions = vec![expired_instruction, create_ata, pump_instruction];
    let build_duration = start_build.elapsed();
    println!("pumpbuy 本地构建花费 {:?}, mint: {:?}, [{}]", build_duration, token_mint, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

    #[cfg(not(test))]
    let sig = tx_pump_buy(&signer, instructions).await?;
    // let snipe_duration = start_build.elapsed();
    // println!("狙击完成总耗时 {:?}", snipe_duration);
    let app_state = APP_STATE.get().expect("AppState not initialized");
    let nonce_pubkey2 = Pubkey::from_str(NONCE_PUBKEY.as_ref()).unwrap();
    let nonce_pubkey = Arc::new(nonce_pubkey2);

    update_nonce(&app_state, nonce_pubkey).await;

    Ok(())
}

pub async fn pump_sell(
    token_mint: Pubkey,
    creator_account: Pubkey,
    token_amount: u64,
) -> Result<(), Error> {
    // let start_build = Instant::now();
    println!("开始出售代币");
    let signer = solana_sdk::signature::Keypair::from_base58_string(PRIVATE_KEY.as_str());

    let (creator_vault, _) = Pubkey::find_program_address(
        &[b"creator-vault", creator_account.as_ref()],
        &PUMP_PROGRAM_ID,
    );

    println!("出售代币: {} ", token_mint);
    println!("代币数量: {:.2} ", token_amount);

    let (bonding_curve_address, associated_bonding_curve) =
        get_bonding_curve_account(&token_mint, &PUMP_PROGRAM_ID).await?;

    let min_sol_receive: u64 = 0;

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(PUMP_SELL_SELECTOR);
    data.extend_from_slice(&token_amount.to_le_bytes());
    data.extend_from_slice(&min_sol_receive.to_le_bytes());

    // 用户代币关联账户
    let associated_user = get_associated_token_address(&signer.pubkey(), &token_mint);

    let pump_instruction = Instruction::new_with_bytes(
        PROXY_PROGRAM,
        &data,
        vec![
            AccountMeta::new_readonly(GLOBAL_ACCOUNT, false),
            AccountMeta::new(FEE_RECIPIENT, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(bonding_curve_address, false),
            AccountMeta::new(associated_bonding_curve, false),
            AccountMeta::new(associated_user, false),
            AccountMeta::new(signer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(creator_vault, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            // AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(EVENT_AUTHORITY, false),
            AccountMeta::new_readonly(PUMP_PROGRAM_ID, false),
        ],
    );

    let instructions = vec![pump_instruction];
    // let build_duration = start_build.elapsed();
    // println!("pumpsell build took {:?}", build_duration);

    #[cfg(not(test))]
    let sig = tx_pump_sell(&signer, instructions).await?;

    Ok(())
}

pub async fn get_bonding_curve_account(
    mint: &Pubkey,
    program_id: &Pubkey,
) -> Result<(Pubkey, Pubkey)> {
    let bonding_curve = get_pda(mint, program_id)?;
    let associated_bonding_curve = get_associated_token_address(&bonding_curve, &mint);

    Ok((bonding_curve, associated_bonding_curve))
}

pub fn get_pda(mint: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
    let seeds = [b"bonding-curve".as_ref(), mint.as_ref()];
    let (bonding_curve, _bump) = Pubkey::find_program_address(&seeds, program_id);
    Ok(bonding_curve)
}
