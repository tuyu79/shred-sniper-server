use crate::models::pump_parser::PumpTransaction;
use std::collections::HashSet;

// 交易结果容器，性能优化版本
#[derive(Default, Debug)]
pub struct TransactionResults {
    pub pump_signatures: HashSet<String>,
    pub pump_transactions: Vec<PumpTransaction>, // 存储PUMP交易的详细信息
    pub current_slot: u64,                       // 存储当前处理的slot
}

impl TransactionResults {
    pub fn new() -> Self {
        Self {
            // 使用较大的初始容量减少重新分配
            pump_signatures: HashSet::with_capacity(512),
            pump_transactions: Vec::with_capacity(256),
            current_slot: 0,
        }
    }

    #[inline(always)]
    pub fn has_results(&self) -> bool {
        !self.pump_transactions.is_empty()
    }

    // 设置当前slot
    #[inline(always)]
    pub fn set_current_slot(&mut self, slot: u64) {
        self.current_slot = slot;
    }

    // 添加一个PUMP交易
    #[inline(always)]
    pub fn add_pump_transaction(&mut self, tx: PumpTransaction) {
        // 确保只添加签名不重复的交易
        if !self.pump_signatures.contains(&tx.signature) {
            self.pump_signatures.insert(tx.signature.clone());
            self.pump_transactions.push(tx);
        }
    }

    // 合并另一个结果集到当前结果集
    #[allow(dead_code)]
    pub fn merge(&mut self, other: TransactionResults) {
        // 提前预留空间，减少重新分配
        self.pump_transactions
            .reserve(other.pump_transactions.len());
        self.pump_signatures.reserve(other.pump_signatures.len());

        for tx in other.pump_transactions {
            self.add_pump_transaction(tx);
        }
    }

    // 批量添加PUMP交易
    #[allow(dead_code)]
    #[inline]
    pub fn add_pump_transactions(&mut self, transactions: Vec<PumpTransaction>) {
        // 提前预留空间，减少重新分配
        self.pump_transactions.reserve(transactions.len());
        self.pump_signatures.reserve(transactions.len());

        for tx in transactions {
            self.add_pump_transaction(tx);
        }
    }
}
