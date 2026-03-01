use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{
    constants::{CONFIG_SEED, STATE_SEED, VAULT_SEED},
    error::VaultError,
    state::{VaultConfig, VaultState},
};

#[derive(Accounts)]
pub struct WithdrawRestricted<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [CONFIG_SEED, user.key().as_ref()],
        bump = vault_config.config_bump,
    )]
    pub vault_config: Account<'info, VaultConfig>,
    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawRestricted<'info> {
    pub fn withdraw_restricted(&mut self, amount: u64) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        require!(
            self.vault.lamports() >= amount,
            VaultError::InsufficientVaultFunds
        );

        if self.vault_config.lock_until_ts > 0 {
            require!(
                now >= self.vault_config.lock_until_ts,
                VaultError::VaultStillLocked
            );
        }

        let mut new_withdrawn_this_period = self.vault_config.withdrawn_this_period;

        if self.vault_config.spend_limit > 0 {
            require!(
                self.vault_config.period_seconds > 0,
                VaultError::PeriodRequiredForSpendLimit
            );

            let period_end = self
                .vault_config
                .period_start_ts
                .checked_add(self.vault_config.period_seconds)
                .ok_or(VaultError::NumericalOverflow)?;

            if now >= period_end {
                self.vault_config.period_start_ts = now;
                self.vault_config.withdrawn_this_period = 0;
                new_withdrawn_this_period = 0;
            }

            new_withdrawn_this_period = new_withdrawn_this_period
                .checked_add(amount)
                .ok_or(VaultError::NumericalOverflow)?;

            require!(
                new_withdrawn_this_period <= self.vault_config.spend_limit,
                VaultError::SpendLimitExceeded
            );
        }

        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let seeds = &[
            VAULT_SEED,
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        transfer(cpi_ctx, amount)?;

        if self.vault_config.spend_limit > 0 {
            self.vault_config.withdrawn_this_period = new_withdrawn_this_period;
        }

        Ok(())
    }
}
