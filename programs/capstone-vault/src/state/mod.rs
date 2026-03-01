use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
}

#[derive(InitSpace)]
#[account]
pub struct VaultConfig {
    pub lock_until_ts: i64,
    pub spend_limit: u64,
    pub period_seconds: i64,
    pub period_start_ts: i64,
    pub withdrawn_this_period: u64,
    pub config_bump: u8,
}
