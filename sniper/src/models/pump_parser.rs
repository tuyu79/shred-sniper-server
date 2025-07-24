use crate::config::MAX_SOL;
use solana_program::instruction::CompiledInstruction;
use solana_sdk::message::VersionedMessage;
use solana_sdk::message::legacy::Message as LegacyMessage;
use solana_sdk::message::v0::Message as V0Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;
use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

// PUMP程序ID
#[allow(dead_code)]
pub const PUMP_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

// PUMP指令类型
#[derive(Debug, PartialEq, Clone)]
#[allow(dead_code)]
pub enum PumpInstructionType {
    Unknown,
    Buy,    // 只关注Buy指令
    Create, // 只关注Create指令
}

// PUMP指令的详细信息
#[derive(Debug, Clone)]
pub struct PumpInstruction {
    pub instruction_type: PumpInstructionType,
    pub accounts: Vec<String>,
    pub data: Vec<u8>,
}

impl fmt::Display for PumpInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.instruction_type {
            PumpInstructionType::Buy => {
                if self.data.len() >= 16 {
                    // 解析amount参数
                    let amount = u64::from_le_bytes([
                        self.data[8],
                        self.data[9],
                        self.data[10],
                        self.data[11],
                        self.data[12],
                        self.data[13],
                        self.data[14],
                        self.data[15],
                    ]);

                    writeln!(f, "Token_Amount: {}", amount)?;

                    // 解析max_sol_cost参数
                    if self.data.len() >= 24 {
                        let max_sol_cost = u64::from_le_bytes([
                            self.data[16],
                            self.data[17],
                            self.data[18],
                            self.data[19],
                            self.data[20],
                            self.data[21],
                            self.data[22],
                            self.data[23],
                        ]);
                        writeln!(f, "Max_SOL_Cost: {} ", max_sol_cost)?;
                    }
                }

                // 打印账户信息
                if self.accounts.len() >= 3 {
                    writeln!(f, "[0]Global: {}", self.accounts[0])?;
                    writeln!(f, "[1]Fee_Recipient: {}", self.accounts[1])?;
                    writeln!(f, "[2]Mint: {}", self.accounts[2])?;

                    if self.accounts.len() >= 4 {
                        writeln!(f, "[3]Bonding_Curve: {}", self.accounts[3])?;
                    }
                    if self.accounts.len() >= 5 {
                        writeln!(f, "[4]Associated_Bonding_Curve: {}", self.accounts[4])?;
                    }
                    if self.accounts.len() >= 6 {
                        writeln!(f, "[5]Associated_User: {}", self.accounts[5])?;
                    }
                    if self.accounts.len() >= 7 {
                        writeln!(f, "[6]User: {}", self.accounts[6])?;
                    }
                    if self.accounts.len() >= 8 {
                        writeln!(f, "[7]System_Program: {}", self.accounts[7])?;
                    }
                    if self.accounts.len() >= 9 {
                        writeln!(f, "[8]Token_Program: {}", self.accounts[8])?;
                    }
                    if self.accounts.len() >= 10 {
                        writeln!(f, "[9]Rent: {}", self.accounts[9])?;
                    }
                    if self.accounts.len() >= 11 {
                        writeln!(f, "[10]Event_Authority: {}", self.accounts[10])?;
                    }
                    if self.accounts.len() >= 12 {
                        writeln!(f, "[11]Program: {}", self.accounts[11])?;
                    }
                }
            }
            PumpInstructionType::Create => {
                // Create指令的字符串参数在data[8..]之后
                if self.data.len() > 8 {
                    // 前8字节是discriminator，后面是参数数据
                    let mut offset = 8;

                    // 解析name字段
                    if offset + 4 <= self.data.len() {
                        // 读取name字符串长度
                        let name_len = u32::from_le_bytes([
                            self.data[offset],
                            self.data[offset + 1],
                            self.data[offset + 2],
                            self.data[offset + 3],
                        ]) as usize;
                        offset += 4;

                        // 读取name字符串内容
                        if offset + name_len <= self.data.len() {
                            let name =
                                String::from_utf8_lossy(&self.data[offset..offset + name_len]);
                            writeln!(f, "name: {}", name)?;
                            offset += name_len;

                            // 解析symbol字段
                            if offset + 4 <= self.data.len() {
                                // 读取symbol字符串长度
                                let symbol_len = u32::from_le_bytes([
                                    self.data[offset],
                                    self.data[offset + 1],
                                    self.data[offset + 2],
                                    self.data[offset + 3],
                                ]) as usize;
                                offset += 4;

                                // 读取symbol字符串内容
                                if offset + symbol_len <= self.data.len() {
                                    let symbol = String::from_utf8_lossy(
                                        &self.data[offset..offset + symbol_len],
                                    );
                                    writeln!(f, "symbol: {}", symbol)?;
                                    offset += symbol_len;

                                    // 解析URI字段
                                    if offset + 4 <= self.data.len() {
                                        // 读取URI字符串长度
                                        let uri_len = u32::from_le_bytes([
                                            self.data[offset],
                                            self.data[offset + 1],
                                            self.data[offset + 2],
                                            self.data[offset + 3],
                                        ])
                                            as usize;
                                        offset += 4;

                                        // 读取URI字符串内容
                                        if offset + uri_len <= self.data.len() {
                                            let uri = String::from_utf8_lossy(
                                                &self.data[offset..offset + uri_len],
                                            );
                                            writeln!(f, "uri: {}", uri)?;
                                            offset += uri_len;

                                            // 解析creator字段 (Pubkey是32字节)
                                            if offset + 32 <= self.data.len() {
                                                let creator =
                                                    bs58::encode(&self.data[offset..offset + 32])
                                                        .into_string();
                                                writeln!(f, "creator: {}", creator)?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 打印账户信息
                if self.accounts.len() >= 3 {
                    writeln!(f, "[0]Mint: {}", self.accounts[0])?;
                    writeln!(f, "[1]Mint_Authority: {}", self.accounts[1])?;
                    writeln!(f, "[2]Bonding_Curve: {}", self.accounts[2])?;

                    if self.accounts.len() >= 4 {
                        writeln!(f, "[3]Associated_Bonding_Curve: {}", self.accounts[3])?;
                    }
                    if self.accounts.len() >= 5 {
                        writeln!(f, "[4]Global: {}", self.accounts[4])?;
                    }
                    if self.accounts.len() >= 6 {
                        writeln!(f, "[5]Mpl_Token_Metadata: {}", self.accounts[5])?;
                    }
                    if self.accounts.len() >= 7 {
                        writeln!(f, "[6]Metadata: {}", self.accounts[6])?;
                    }
                    if self.accounts.len() >= 8 {
                        writeln!(f, "[7]User: {}", self.accounts[7])?;
                    }
                    if self.accounts.len() >= 9 {
                        writeln!(f, "[8]System_Program: {}", self.accounts[8])?;
                    }
                    if self.accounts.len() >= 10 {
                        writeln!(f, "[9]Token_Program: {}", self.accounts[9])?;
                    }
                    if self.accounts.len() >= 11 {
                        writeln!(f, "[10]Associated_Token_Program: {}", self.accounts[10])?;
                    }
                    if self.accounts.len() >= 12 {
                        writeln!(f, "[11]Rent: {}", self.accounts[11])?;
                    }
                    if self.accounts.len() >= 13 {
                        writeln!(f, "[12]Event_Authority: {}", self.accounts[12])?;
                    }
                    if self.accounts.len() >= 14 {
                        writeln!(f, "[13]Program: {}", self.accounts[13])?;
                    }
                }
            }
            PumpInstructionType::Unknown => {
                // 不打印未知指令的详细信息
            }
        }

        Ok(())
    }
}

// PUMP交易的解析结果
#[derive(Debug, Clone)]
pub struct PumpTransaction {
    pub signature: String,
    pub mint: String,
    pub bonding_curve: String,
    pub associated_bonding_curve: String,
    pub creator: String,
    pub price: f64,
    pub buy_amount: u64,
    pub max_sol_cost: u64,
    pub my_token_amount: u64,
    pub instructions: Vec<PumpInstruction>,
}

// 添加用于计算债券曲线的结构体
#[derive(Debug, Clone)]
pub struct TokenReserves {
    pub virtual_sol_reserves: u64,   // 虚拟SOL储备
    pub virtual_token_reserves: u64, // 虚拟代币储备
}

impl TokenReserves {
    // 计算当前价格 (SOL/Token)，考虑代币精度
    pub fn calculate_price(&self) -> f64 {
        if self.virtual_token_reserves == 0 {
            return 0.0;
        }

        // 直接计算实际价格比例，不考虑精度因子
        // 这样显示的是原始比例，便于查看实际数值
        (self.virtual_sol_reserves as f64) / (self.virtual_token_reserves as f64)
    }

    // 计算购买指定数量代币所需的SOL
    pub fn calculate_sol_cost(&self, token_amount: u64) -> f64 {
        if token_amount == 0 || self.virtual_token_reserves == 0 {
            return 0.0;
        }

        let k = (self.virtual_sol_reserves as f64) * (self.virtual_token_reserves as f64);
        let new_token_reserves = self.virtual_token_reserves.saturating_sub(token_amount) as f64;
        if new_token_reserves <= 0.0 {
            return f64::MAX;
        }

        let new_sol_reserves = k / new_token_reserves;
        new_sol_reserves - (self.virtual_sol_reserves as f64)
    }

    // 计算销售指定数量代币获得的SOL
    pub fn calculate_sol_return(&self, token_amount: u64) -> f64 {
        if token_amount == 0 {
            return 0.0;
        }

        let k = (self.virtual_sol_reserves as f64) * (self.virtual_token_reserves as f64);
        let new_token_reserves = (self.virtual_token_reserves as f64) + (token_amount as f64);
        let new_sol_reserves = k / new_token_reserves;

        (self.virtual_sol_reserves as f64) - new_sol_reserves
    }
}

impl fmt::Display for PumpTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 检查是否有Create指令和Buy指令（这个检查在transaction_processor中已经提前完成）
        // 提前获取指令引用，避免重复迭代
        let mut create_instructions = Vec::with_capacity(1);
        let mut buy_instructions = Vec::with_capacity(1);

        for ix in &self.instructions {
            match ix.instruction_type {
                PumpInstructionType::Create => create_instructions.push(ix),
                PumpInstructionType::Buy => buy_instructions.push(ix),
                _ => {}
            }
        }

        // 如果没有足够的指令类型，提前返回
        if create_instructions.is_empty() || buy_instructions.is_empty() {
            return Ok(());
        }

        // 假设现在只有一个Create指令和一个Buy指令
        let create_ix = &create_instructions[0];
        let buy_ix = &buy_instructions[0];

        // 从Create指令中提取元数据信息
        let mut name = String::new();
        let mut symbol = String::new();
        let mut uri = String::new();
        let mut creator = String::new();
        let mut mint = String::new();
        let mut bonding_curve = String::new();
        let mut associated_boding_curve = String::new();
        let mut user_address = String::new();

        // 提取元数据
        if create_ix.data.len() > 8 {
            let mut offset = 8;

            // 读取name
            if offset + 4 <= create_ix.data.len() {
                let name_len = u32::from_le_bytes([
                    create_ix.data[offset],
                    create_ix.data[offset + 1],
                    create_ix.data[offset + 2],
                    create_ix.data[offset + 3],
                ]) as usize;
                offset += 4;

                if offset + name_len <= create_ix.data.len() {
                    name = String::from_utf8_lossy(&create_ix.data[offset..offset + name_len])
                        .to_string();
                    offset += name_len;

                    // 读取symbol
                    if offset + 4 <= create_ix.data.len() {
                        let symbol_len = u32::from_le_bytes([
                            create_ix.data[offset],
                            create_ix.data[offset + 1],
                            create_ix.data[offset + 2],
                            create_ix.data[offset + 3],
                        ]) as usize;
                        offset += 4;

                        if offset + symbol_len <= create_ix.data.len() {
                            symbol = String::from_utf8_lossy(
                                &create_ix.data[offset..offset + symbol_len],
                            )
                            .to_string();
                            offset += symbol_len;

                            // 读取URI
                            if offset + 4 <= create_ix.data.len() {
                                let uri_len = u32::from_le_bytes([
                                    create_ix.data[offset],
                                    create_ix.data[offset + 1],
                                    create_ix.data[offset + 2],
                                    create_ix.data[offset + 3],
                                ]) as usize;
                                offset += 4;

                                if offset + uri_len <= create_ix.data.len() {
                                    uri = String::from_utf8_lossy(
                                        &create_ix.data[offset..offset + uri_len],
                                    )
                                    .to_string();
                                    offset += uri_len;

                                    // 读取creator
                                    if offset + 32 <= create_ix.data.len() {
                                        creator =
                                            bs58::encode(&create_ix.data[offset..offset + 32])
                                                .into_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 提取Mint、BondingCurve和User地址
        if create_ix.accounts.len() >= 3 {
            mint = create_ix.accounts[0].clone();
            bonding_curve = create_ix.accounts[2].clone();
            associated_boding_curve = create_ix.accounts[3].clone();

            if create_ix.accounts.len() >= 8 {
                user_address = create_ix.accounts[7].clone();
            }
        }

        // 从Buy指令中提取交易信息
        let mut token_amount = 0u64;
        let mut max_sol_cost = 0u64;

        if buy_ix.data.len() >= 16 {
            token_amount = u64::from_le_bytes([
                buy_ix.data[8],
                buy_ix.data[9],
                buy_ix.data[10],
                buy_ix.data[11],
                buy_ix.data[12],
                buy_ix.data[13],
                buy_ix.data[14],
                buy_ix.data[15],
            ]);

            if buy_ix.data.len() >= 24 {
                max_sol_cost = u64::from_le_bytes([
                    buy_ix.data[16],
                    buy_ix.data[17],
                    buy_ix.data[18],
                    buy_ix.data[19],
                    buy_ix.data[20],
                    buy_ix.data[21],
                    buy_ix.data[22],
                    buy_ix.data[23],
                ]);
            }
        }

        // 使用当前时间作为时间戳
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 初始默认值
        let initial_sol_reserves: u64 = 30_000_000_000;
        let initial_token_reserves: u64 = 1_073_000_000_000_000;

        // 计算实际储备值 - 如果已经购买代币，储备会发生变化
        // 使用恒定乘积公式 k = sol_reserves * token_reserves
        let k = (initial_sol_reserves as f64) * (initial_token_reserves as f64);

        // 根据代币数量估算当前储备情况
        // 当token_amount非常小时，几乎等于初始值
        // 当token_amount较大时，将明显影响储备
        let current_token_reserves = if token_amount == 0 {
            initial_token_reserves
        } else {
            initial_token_reserves.saturating_sub(token_amount)
        };

        // 根据恒定乘积公式，计算对应的SOL储备
        let current_sol_reserves = if current_token_reserves == 0 {
            initial_sol_reserves // 避免除以零
        } else {
            (k / (current_token_reserves as f64)) as u64
        };

        // 正确计算价格，考虑不同精度
        // SOL精度为1e9，代币精度为1e6
        const SOL_DECIMALS: f64 = 1_000_000_000.0; // 10^9
        const TOKEN_DECIMALS: f64 = 1_000_000.0; // 10^6

        // 计算实际SOL和代币的比例（考虑精度）
        let price_in_sol = (current_sol_reserves as f64 / SOL_DECIMALS)
            / (current_token_reserves as f64 / TOKEN_DECIMALS);

        let my_max_sol_cost: u64 = 1200_000_000; // lamports 0.1 sol
        // // 1. 把 lamports 转成 SOL
        let max_sol = my_max_sol_cost as f64 / 1_000_000_000.0;
        let precision_factor = 1_000_000.0;

        let my_buy_token_amount: u64 = (max_sol as f64 / price_in_sol) as u64;
        // 减少15%的购买数量，以避免滑点错误
        let reduced_amount = (my_buy_token_amount as f64 * 0.92) as u64;
        let my_token_amount = (reduced_amount as f64 * precision_factor).floor() as u64;

        // 按照要求格式输出
        // writeln!(f, "timestamp: {}", timestamp)?;
        writeln!(f, "signature: {}", self.signature)?;
        // writeln!(f, "mint: {}", mint)?;
        // writeln!(f, "bonding_curve: {}", bonding_curve)?;
        // writeln!(f, "associated_bonding_curve: {}", associated_boding_curve)?;
        // writeln!(f, "user: {}", user_address)?;
        // writeln!(f, "token_amount: {}", token_amount)?;
        writeln!(f, "max_sol_cost: {}", max_sol_cost)?;
        // writeln!(f, "sol_reserves: {}", current_sol_reserves)?;
        // writeln!(f, "token_reserves: {}", current_token_reserves)?;
        // writeln!(f, "price: {:.12}", price_in_sol)?;
        writeln!(f, "name: {}", name)?;
        writeln!(f, "symbol: {}", symbol)?;
        // writeln!(f, "uri: {}", uri)?;
        // writeln!(f, "creator: {}", creator)?;

        Ok(())
    }
}

// PUMP解析器
#[allow(dead_code)]
pub struct PumpParser;

impl PumpParser {
    // 解析交易，提取PUMP指令
    #[allow(dead_code)]
    pub fn parse_transaction(transaction: &VersionedTransaction) -> Option<PumpTransaction> {
        // 获取PUMP程序的Pubkey
        let pump_program_id = Pubkey::from_str(PUMP_PROGRAM_ID).ok()?;

        // 提取PUMP相关指令
        let pump_instructions = match &transaction.message {
            VersionedMessage::Legacy(message) => {
                Self::extract_pump_instructions_from_legacy(message, &pump_program_id)
            }
            VersionedMessage::V0(message) => {
                Self::extract_pump_instructions_from_v0(message, &pump_program_id)
            }
        };

        // 如果没有找到PUMP指令，返回None
        if pump_instructions.is_empty() {
            return None;
        }

        // 获取交易签名
        let signature = if !transaction.signatures.is_empty() {
            transaction.signatures[0].to_string()
        } else {
            "No_Signature".to_string()
        };

        // 分离Create和Buy指令
        let mut create_instruction: Option<&PumpInstruction> = None;
        let mut buy_instruction: Option<&PumpInstruction> = None;

        for ix in &pump_instructions {
            match ix.instruction_type {
                PumpInstructionType::Create => create_instruction = Some(ix),
                PumpInstructionType::Buy => buy_instruction = Some(ix),
                _ => {}
            }
        }

        // 确保有Create和Buy指令（根据需求调整）
        let create_ix = create_instruction?;
        let buy_ix = buy_instruction?;

        // 从Create指令中提取元数据信息
        let mut name = String::new();
        let mut symbol = String::new();
        let mut uri = String::new();
        let mut creator = String::new();
        let mut mint = String::new();
        let mut bonding_curve = String::new();
        let mut associated_bonding_curve = String::new();
        let mut user_address = String::new();

        // 提取Mint、BondingCurve和User地址
        if create_ix.accounts.len() >= 3 {
            mint = create_ix.accounts[0].clone();
            bonding_curve = create_ix.accounts[2].clone();
            associated_bonding_curve = create_ix.accounts[3].clone();

            if create_ix.accounts.len() >= 8 {
                user_address = create_ix.accounts[7].clone();
            }
        }

        // 提取元数据
        if create_ix.data.len() > 8 {
            let mut offset = 8;

            // 读取name
            if offset + 4 <= create_ix.data.len() {
                let name_len = u32::from_le_bytes([
                    create_ix.data[offset],
                    create_ix.data[offset + 1],
                    create_ix.data[offset + 2],
                    create_ix.data[offset + 3],
                ]) as usize;
                offset += 4;

                if offset + name_len <= create_ix.data.len() {
                    name = String::from_utf8_lossy(&create_ix.data[offset..offset + name_len])
                        .to_string();
                    offset += name_len;

                    // 读取symbol
                    if offset + 4 <= create_ix.data.len() {
                        let symbol_len = u32::from_le_bytes([
                            create_ix.data[offset],
                            create_ix.data[offset + 1],
                            create_ix.data[offset + 2],
                            create_ix.data[offset + 3],
                        ]) as usize;
                        offset += 4;

                        if offset + symbol_len <= create_ix.data.len() {
                            symbol = String::from_utf8_lossy(
                                &create_ix.data[offset..offset + symbol_len],
                            )
                            .to_string();
                            offset += symbol_len;

                            // 读取URI
                            if offset + 4 <= create_ix.data.len() {
                                let uri_len = u32::from_le_bytes([
                                    create_ix.data[offset],
                                    create_ix.data[offset + 1],
                                    create_ix.data[offset + 2],
                                    create_ix.data[offset + 3],
                                ]) as usize;
                                offset += 4;

                                if offset + uri_len <= create_ix.data.len() {
                                    uri = String::from_utf8_lossy(
                                        &create_ix.data[offset..offset + uri_len],
                                    )
                                    .to_string();
                                    offset += uri_len;

                                    // 读取creator
                                    if offset + 32 <= create_ix.data.len() {
                                        creator =
                                            bs58::encode(&create_ix.data[offset..offset + 32])
                                                .into_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 从Buy指令提取buy_amount和max_sol_cost
        let mut buy_amount = 0u64;
        let mut max_sol_cost = 0u64;

        if buy_ix.data.len() >= 16 {
            buy_amount = u64::from_le_bytes([
                buy_ix.data[8],
                buy_ix.data[9],
                buy_ix.data[10],
                buy_ix.data[11],
                buy_ix.data[12],
                buy_ix.data[13],
                buy_ix.data[14],
                buy_ix.data[15],
            ]);

            if buy_ix.data.len() >= 24 {
                max_sol_cost = u64::from_le_bytes([
                    buy_ix.data[16],
                    buy_ix.data[17],
                    buy_ix.data[18],
                    buy_ix.data[19],
                    buy_ix.data[20],
                    buy_ix.data[21],
                    buy_ix.data[22],
                    buy_ix.data[23],
                ]);
            }
        } else {
            return None; // Buy指令数据不足
        }

        // 计算价格
        const SOL_DECIMALS: f64 = 1_000_000_000.0; // 10^9
        const TOKEN_DECIMALS: f64 = 1_000_000.0; // 10^6
        let initial_sol_reserves: u64 = 30_000_000_000; // 30 SOL in lamports
        let initial_token_reserves: u64 = 1_073_000_000_000_000; // 1.073B tokens

        let k = (initial_sol_reserves as f64) * (initial_token_reserves as f64);
        let current_token_reserves = initial_token_reserves.saturating_sub(buy_amount);
        let current_sol_reserves = if current_token_reserves == 0 {
            initial_sol_reserves
        } else {
            (k / (current_token_reserves as f64)) as u64
        };

        let price = if current_token_reserves == 0 {
            0.0
        } else {
            (current_sol_reserves as f64 / SOL_DECIMALS)
                / (current_token_reserves as f64 / TOKEN_DECIMALS)
        };

        unsafe {
            let max_sol = *MAX_SOL;
            let precision_factor = 1_000_000.0;

            let my_buy_token_amount: u64 = (max_sol as f64 / price) as u64;
            // 减少15%的购买数量，以避免滑点错误
            let reduced_amount = (my_buy_token_amount as f64 * 0.94) as u64;
            let my_token_amount = (reduced_amount as f64 * precision_factor).floor() as u64;

            // 构造PumpTransaction
            Some(PumpTransaction {
                signature,
                mint,
                bonding_curve,
                associated_bonding_curve,
                creator,
                price,
                buy_amount,
                max_sol_cost,
                my_token_amount,
                instructions: pump_instructions,
            })
        }
    }

    // 从Legacy消息中提取PUMP指令
    fn extract_pump_instructions_from_legacy(
        message: &LegacyMessage,
        pump_program_id: &Pubkey,
    ) -> Vec<PumpInstruction> {
        let account_keys = &message.account_keys;

        message
            .instructions
            .iter()
            .filter_map(|ix| {
                let program_idx = ix.program_id_index as usize;
                if program_idx < account_keys.len() && account_keys[program_idx] == *pump_program_id
                {
                    Some(Self::compile_instruction_to_pump_instruction(
                        ix,
                        account_keys,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    // 从V0消息中提取PUMP指令
    fn extract_pump_instructions_from_v0(
        message: &V0Message,
        pump_program_id: &Pubkey,
    ) -> Vec<PumpInstruction> {
        // 获取静态账户
        let static_keys = &message.account_keys;

        // 如果有地址查找表，简化处理（只处理静态账户）
        if !message.address_table_lookups.is_empty() {
            return Vec::new();
        }

        message
            .instructions
            .iter()
            .filter_map(|ix| {
                if ix.program_id_index as usize >= static_keys.len() {
                    return None;
                }

                let program_id = &static_keys[ix.program_id_index as usize];

                if program_id == pump_program_id {
                    Some(Self::compile_instruction_to_pump_instruction(
                        ix,
                        static_keys,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    // 将编译后的指令转换为PUMP指令
    fn compile_instruction_to_pump_instruction(
        ix: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> PumpInstruction {
        // 解析指令类型
        let instruction_type = if !ix.data.is_empty() && ix.data.len() >= 8 {
            // 根据discriminator识别指令类型，只处理Buy和Create类型
            let discriminator = &ix.data[0..8];
            match discriminator {
                // Buy指令
                [102, 6, 61, 18, 1, 218, 235, 234] => PumpInstructionType::Buy,
                [242, 35, 198, 137, 82, 225, 242, 182] => PumpInstructionType::Buy,

                // Create指令
                [24, 30, 200, 40, 5, 28, 7, 119] => PumpInstructionType::Create,
                [54, 49, 138, 255, 162, 99, 87, 199] => PumpInstructionType::Create,

                // 其他指令都归类为Unknown
                _ => PumpInstructionType::Unknown,
            }
        } else {
            PumpInstructionType::Unknown
        };

        // 获取账户地址
        let accounts = ix
            .accounts
            .iter()
            .filter_map(|account_idx| {
                account_keys
                    .get(*account_idx as usize)
                    .map(|pubkey| pubkey.to_string())
            })
            .collect();

        PumpInstruction {
            instruction_type,
            accounts,
            data: ix.data.clone(),
        }
    }
}
