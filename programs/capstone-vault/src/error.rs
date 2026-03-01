use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Vault is still time-locked.")]
    VaultStillLocked,
    #[msg("Withdrawal exceeds spend limit for current period.")]
    SpendLimitExceeded,
    #[msg("Spend limit requires a positive period.")]
    PeriodRequiredForSpendLimit,
    #[msg("Invalid lock duration.")]
    InvalidLockDuration,
    #[msg("Invalid spend limit configuration.")]
    InvalidSpendLimitConfig,
    #[msg("Insufficient funds in vault.")]
    InsufficientVaultFunds,
    #[msg("Numerical overflow.")]
    NumericalOverflow,
}
