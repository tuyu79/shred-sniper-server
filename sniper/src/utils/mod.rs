// 工具函数模块，用于放置通用的工具函数
use std::collections::HashMap;

// 格式化显示进度条
#[allow(dead_code)]
pub fn format_progress(current: usize, total: usize, width: usize) -> String {
    let percent = current as f64 / total as f64;
    let filled_width = (width as f64 * percent) as usize;
    let empty_width = width - filled_width;

    let filled = "=".repeat(filled_width);
    let empty = " ".repeat(empty_width);

    format!("[{}{}] {:.1}%", filled, empty, percent * 100.0)
}

// PUMP_AMM Buy指令账户索引标签映射
#[allow(dead_code)]
pub fn get_pumpamm_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Pool");
    labels.insert(1, "User");
    labels.insert(2, "Global_Config");
    labels.insert(3, "Base_Mint");
    labels.insert(4, "Quote_Mint");
    labels.insert(5, "User_Base_Token_Account");
    labels.insert(6, "User_Quote_Token_Account");
    labels.insert(7, "Pool_Base_Token_Account");
    labels.insert(8, "Pool_Quote_Token_Account");
    labels.insert(9, "Protocol_Fee_Recipient");
    labels.insert(10, "Protocol_Fee_Recipient_Token_Account");
    labels.insert(11, "Base_Token_Program");
    labels.insert(12, "Quote_Token_Program");
    labels.insert(13, "System_Program");
    labels.insert(14, "Associated_Token_Program");
    labels.insert(15, "Event_Authority");
    labels.insert(16, "Program");

    labels
}

// PUMP_AMM Sell指令账户索引标签映射
#[allow(dead_code)]
pub fn get_pumpamm_sell_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Pool");
    labels.insert(1, "User");
    labels.insert(2, "Global_Config");
    labels.insert(3, "Base_Mint");
    labels.insert(4, "Quote_Mint");
    labels.insert(5, "User_Base_Token_Account");
    labels.insert(6, "User_Quote_Token_Account");
    labels.insert(7, "Pool_Base_Token_Account");
    labels.insert(8, "Pool_Quote_Token_Account");
    labels.insert(9, "Protocol_Fee_Recipient");
    labels.insert(10, "Protocol_Fee_Recipient_Token_Account");
    labels.insert(11, "Base_Token_Program");
    labels.insert(12, "Quote_Token_Program");
    labels.insert(13, "System_Program");
    labels.insert(14, "Event_Authority");
    labels.insert(15, "Program");

    labels
}

// PUMP_AMM CreatePool指令账户索引标签映射
#[allow(dead_code)]
pub fn get_pumpamm_createpool_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Pool");
    labels.insert(1, "LP_Mint");
    labels.insert(2, "User");
    labels.insert(3, "Global_Config");
    labels.insert(4, "Base_Mint");
    labels.insert(5, "Quote_Mint");
    labels.insert(6, "User_Base_Token_Account");
    labels.insert(7, "User_Quote_Token_Account");
    labels.insert(8, "Pool_Base_Token_Account");
    labels.insert(9, "Pool_Quote_Token_Account");
    labels.insert(10, "User_LP_Token_Account");
    labels.insert(11, "Base_Token_Program");
    labels.insert(12, "Quote_Token_Program");
    labels.insert(13, "Rent");
    labels.insert(14, "System_Program");
    labels.insert(15, "Token_Program");
    labels.insert(16, "Associated_Token_Program");
    labels.insert(17, "Event_Authority");
    labels.insert(18, "Program");

    labels
}

// PUMP_AMM Deposit指令账户索引标签映射
#[allow(dead_code)]
pub fn get_pumpamm_deposit_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Pool");
    labels.insert(1, "LP_Mint");
    labels.insert(2, "User");
    labels.insert(3, "Global_Config");
    labels.insert(4, "Base_Mint");
    labels.insert(5, "Quote_Mint");
    labels.insert(6, "User_Base_Token_Account");
    labels.insert(7, "User_Quote_Token_Account");
    labels.insert(8, "Pool_Base_Token_Account");
    labels.insert(9, "Pool_Quote_Token_Account");
    labels.insert(10, "User_LP_Token_Account");
    labels.insert(11, "Base_Token_Program");
    labels.insert(12, "Quote_Token_Program");
    labels.insert(13, "Token_Program");
    labels.insert(14, "Associated_Token_Program");
    labels.insert(15, "Event_Authority");
    labels.insert(16, "Program");

    labels
}

// PUMP_AMM Withdraw指令账户索引标签映射
#[allow(dead_code)]
pub fn get_pumpamm_withdraw_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Pool");
    labels.insert(1, "LP Mint");
    labels.insert(2, "User");
    labels.insert(3, "Global_Config");
    labels.insert(4, "Base_Mint");
    labels.insert(5, "Quote_Mint");
    labels.insert(6, "User_Base_Token_Account");
    labels.insert(7, "User_Quote_Token_Account");
    labels.insert(8, "Pool_Base_Token_Account");
    labels.insert(9, "Pool_Quote_Token_Account");
    labels.insert(10, "User_LP_Token_Account");
    labels.insert(11, "Base_Token_Program");
    labels.insert(12, "Quote_Token_Program");
    labels.insert(13, "Token_Program");
    labels.insert(14, "Event_Authority");
    labels.insert(15, "Program");

    labels
}

// PUMP_AMM CreateConfig指令账户索引标签映射
#[allow(dead_code)]
pub fn get_pumpamm_createconfig_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Global_Config");
    labels.insert(1, "Fee_Recipient");
    labels.insert(2, "Admin");
    labels.insert(3, "System_Program");
    labels.insert(4, "Rent");
    labels.insert(5, "Event_Authority");
    labels.insert(6, "Program");

    labels
}

// PUMP_AMM UpdateFeeConfig指令账户索引标签映射
#[allow(dead_code)]
pub fn get_pumpamm_updatefeeconfig_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Global_Config");
    labels.insert(1, "Admin");
    labels.insert(2, "Event_Authority");
    labels.insert(3, "Program");

    labels
}

// PUMP指令账户索引标签映射 - Buy指令
#[allow(dead_code)]
pub fn get_pump_buy_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Global");
    labels.insert(1, "Fee_Recipient");
    labels.insert(2, "Mint");
    labels.insert(3, "Bonding_Curve");
    labels.insert(4, "Associated_Bonding_Curve");
    labels.insert(5, "Associated_User");
    labels.insert(6, "User");
    labels.insert(7, "System_Program");
    labels.insert(8, "Token_Program");
    labels.insert(9, "Rent");
    labels.insert(10, "Event_Authority");
    labels.insert(11, "Program");

    labels
}

// PUMP指令账户索引标签映射 - Sell指令
#[allow(dead_code)]
pub fn get_pump_sell_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Global");
    labels.insert(1, "Fee_Recipient");
    labels.insert(2, "Mint");
    labels.insert(3, "Bonding_Curve");
    labels.insert(4, "Associated_Bonding_Curve");
    labels.insert(5, "User");
    labels.insert(6, "Token_Program");
    labels.insert(7, "Event_Authority");
    labels.insert(8, "Program");

    labels
}

// PUMP指令账户索引标签映射 - Create指令
#[allow(dead_code)]
pub fn get_pump_create_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Mint");
    labels.insert(1, "Mint_Authority");
    labels.insert(2, "Bonding_Curve");
    labels.insert(3, "Associated_Bonding_Curve");
    labels.insert(4, "Global");
    labels.insert(5, "Mpl_Token_Metadata");
    labels.insert(6, "Metadata");
    labels.insert(7, "User");
    labels.insert(8, "System_Program");
    labels.insert(9, "Token_Program");
    labels.insert(10, "Associated_Token_Program");
    labels.insert(11, "Rent");
    labels.insert(12, "Event Authority");
    labels.insert(13, "Program");

    labels
}

// PUMP指令账户索引标签映射 - Initialize指令
#[allow(dead_code)]
pub fn get_pump_initialize_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Global");
    labels.insert(1, "Admin");
    labels.insert(2, "Fee_Recipient");
    labels.insert(3, "System_Program");
    labels.insert(4, "Rent");
    labels.insert(5, "Event_Authority");
    labels.insert(6, "Program");

    labels
}

// 根据指令类型获取PUMP_AMM账户标签映射
#[allow(dead_code)]
pub fn get_pumpamm_account_labels_by_instruction(
    instruction_type: &str,
) -> HashMap<usize, &'static str> {
    match instruction_type {
        "Buy" => get_pumpamm_account_labels(),
        "Sell" => get_pumpamm_sell_account_labels(),
        "CreatePool" => get_pumpamm_createpool_account_labels(),
        "Deposit" => get_pumpamm_deposit_account_labels(),
        "Withdraw" => get_pumpamm_withdraw_account_labels(),
        "CreateConfig" => get_pumpamm_createconfig_account_labels(),
        "UpdateFeeConfig" => get_pumpamm_updatefeeconfig_account_labels(),
        _ => HashMap::new(), // 对于未知指令类型返回空映射
    }
}

// 根据指令类型获取PUMP账户标签映射
#[allow(dead_code)]
pub fn get_pump_account_labels_by_instruction(
    instruction_type: &str,
) -> HashMap<usize, &'static str> {
    match instruction_type {
        "Buy" => get_pump_buy_account_labels(),
        "Sell" => get_pump_sell_account_labels(),
        "Create" => get_pump_create_account_labels(),
        "Initialize" => get_pump_initialize_account_labels(),
        _ => HashMap::new(), // 对于未知指令类型返回空映射
    }
}

// boop指令账户索引标签映射 - BuyToken指令
#[allow(dead_code)]
pub fn get_boop_buy_token_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Mint");
    labels.insert(1, "Bonding Curve");
    labels.insert(2, "Trading_Fees_Vault");
    labels.insert(3, "Bonding_Curve_Vault");
    labels.insert(4, "Bonding_Curve_Sol_Vault");
    labels.insert(5, "Recipient_Token_Account");
    labels.insert(6, "Buyer");
    labels.insert(7, "Config");
    labels.insert(8, "Vault_Authority");
    labels.insert(9, "Wsol");
    labels.insert(10, "System_Program");
    labels.insert(11, "Token_Program");
    labels.insert(12, "Associated_Token_Program");

    labels
}

// boop指令账户索引标签映射 - SellToken指令
#[allow(dead_code)]
pub fn get_boop_sell_token_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    labels.insert(0, "Mint");
    labels.insert(1, "Bonding_Curve");
    labels.insert(2, "Trading_Fees_Vault");
    labels.insert(3, "Bonding_Curve_Vault");
    labels.insert(4, "Bonding_Curve_Sol_Vault");
    labels.insert(5, "Seller_Token_Account");
    labels.insert(6, "Seller");
    labels.insert(7, "Recipient");
    labels.insert(8, "Config");
    labels.insert(9, "System_Program");
    labels.insert(10, "Token_Program");
    labels.insert(11, "Associated Token Program");

    labels
}

// boop指令账户索引标签映射 - CreateToken指令
#[allow(dead_code)]
pub fn get_boop_create_token_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    // 使用与 Create 相同的账户标签
    labels.insert(0, "Mint");
    labels.insert(1, "Mint_Authority");
    labels.insert(2, "Payer");
    labels.insert(3, "Config");
    labels.insert(4, "Rent");
    labels.insert(5, "Metadata");
    labels.insert(6, "System_Program");
    labels.insert(7, "Token_Program");
    labels.insert(8, "Token_Metadata_Program");

    labels
}

// boop指令账户索引标签映射 - DeployBondingCurve指令
#[allow(dead_code)]
pub fn get_boop_deploy_bonding_curve_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    // 根据新提供的索引更新标签
    labels.insert(0, "Mint");
    labels.insert(1, "Vault_Authority");
    labels.insert(2, "Bonding_Curve");
    labels.insert(3, "Bonding_Curve_Sol_Vault");
    labels.insert(4, "Bonding_Curve_Vault");
    labels.insert(5, "Config");
    labels.insert(6, "Payer");
    labels.insert(7, "System_Program");
    labels.insert(8, "Token_Program");
    labels.insert(9, "Associated_Token_Program");

    labels
}

// 根据指令类型获取boop账户标签映射
#[allow(dead_code)]
pub fn get_boop_account_labels_by_instruction(
    instruction_type: &str,
) -> HashMap<usize, &'static str> {
    match instruction_type {
        "BuyToken" => get_boop_buy_token_account_labels(),
        "SellToken" => get_boop_sell_token_account_labels(),
        "CreateToken" => get_boop_create_token_account_labels(),
        "DeployBondingCurve" => get_boop_deploy_bonding_curve_account_labels(),
        "Create" => get_boop_create_account_labels(),
        "Sell" => get_boop_sell_account_labels(),
        "Initialize" => get_boop_initialize_account_labels(),
        "SetParams" => get_boop_setparams_account_labels(),
        "UpdateAuthority" => get_boop_updateauthority_account_labels(),
        _ => HashMap::new(), // 对于未知指令类型返回空映射
    }
}

// boop指令账户索引标签映射 - Create指令（原始）
#[allow(dead_code)]
pub fn get_boop_create_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    // 根据新提供的索引更新标签
    labels.insert(0, "Config");
    labels.insert(1, "Metadata");
    labels.insert(2, "Mint");
    labels.insert(3, "Payer");
    labels.insert(4, "Rent");
    labels.insert(5, "System_Program");
    labels.insert(6, "Token_Program");
    labels.insert(7, "Token_Metadata_Program");

    labels
}

// boop指令账户索引标签映射 - Sell指令（原始）
#[allow(dead_code)]
pub fn get_boop_sell_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    // 根据已有的Sell指令输出添加适当的标签
    labels.insert(0, "Global");
    labels.insert(1, "Fee_Recipient");
    labels.insert(2, "Mint");
    labels.insert(3, "Bonding_Curve");
    labels.insert(4, "Associated_Bonding_Curve");
    labels.insert(5, "User");
    labels.insert(6, "Token_Program");
    labels.insert(7, "Event_Authority");
    labels.insert(8, "Program");

    labels
}

// boop指令账户索引标签映射 - Initialize指令
#[allow(dead_code)]
pub fn get_boop_initialize_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    // 添加Initialize指令的账户标签
    labels.insert(0, "Global");
    labels.insert(1, "Admin");
    labels.insert(2, "Fee_Recipient");
    labels.insert(3, "System_Program");
    labels.insert(4, "Rent");
    labels.insert(5, "Event_Authority");
    labels.insert(6, "Program");

    labels
}

// boop指令账户索引标签映射 - SetParams指令
#[allow(dead_code)]
pub fn get_boop_setparams_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    // 添加SetParams指令的账户标签
    labels.insert(0, "Global");
    labels.insert(1, "Admin");
    labels.insert(2, "Event_Authority");
    labels.insert(3, "Program");

    labels
}

// boop指令账户索引标签映射 - UpdateAuthority指令
#[allow(dead_code)]
pub fn get_boop_updateauthority_account_labels() -> HashMap<usize, &'static str> {
    let mut labels = HashMap::new();

    // 添加UpdateAuthority指令的账户标签
    labels.insert(0, "Global");
    labels.insert(1, "Admin");
    labels.insert(2, "New_Admin");
    labels.insert(3, "Event_Authority");
    labels.insert(4, "Program");

    labels
}

// 可以在此添加更多通用工具函数
