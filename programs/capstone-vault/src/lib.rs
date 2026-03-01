use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use constants::{CONFIG_SEED, STATE_SEED, VAULT_SEED};
use error::VaultError;
use instructions::*;
use state::{VaultConfig, VaultState};

declare_id!("FQSsCaaeyjJSb8uZi3hsWkKSJNM3CH9eHahqZy5FapW1");

#[program]
pub mod capstone_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn initialize_restricted(
        ctx: Context<InitializeRestricted>,
        lock_duration_seconds: i64,
        spend_limit: u64,
        spend_period_seconds: i64,
    ) -> Result<()> {
        ctx.accounts.initialize_restricted(
            &ctx.bumps,
            lock_duration_seconds,
            spend_limit,
            spend_period_seconds,
        )
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn withdraw_restricted(ctx: Context<WithdrawRestricted>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw_restricted(amount)
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()
    }

    pub fn close_restricted(ctx: Context<CloseRestricted>) -> Result<()> {
        ctx.accounts.close_restricted()
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump,
        space = VaultState::DISCRIMINATOR.len() + VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());

        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, rent_exempt)?;

        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeRestricted<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump,
        space = VaultState::DISCRIMINATOR.len() + VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        init,
        payer = user,
        seeds = [CONFIG_SEED, user.key().as_ref()],
        bump,
        space = VaultConfig::DISCRIMINATOR.len() + VaultConfig::INIT_SPACE,
    )]
    pub vault_config: Account<'info, VaultConfig>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeRestricted<'info> {
    pub fn initialize_restricted(
        &mut self,
        bumps: &InitializeRestrictedBumps,
        lock_duration_seconds: i64,
        spend_limit: u64,
        spend_period_seconds: i64,
    ) -> Result<()> {
        require!(lock_duration_seconds >= 0, VaultError::InvalidLockDuration);

        if spend_limit > 0 {
            require!(
                spend_period_seconds > 0,
                VaultError::PeriodRequiredForSpendLimit
            );
        } else {
            require!(
                spend_period_seconds == 0,
                VaultError::InvalidSpendLimitConfig
            );
        }

        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());

        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, rent_exempt)?;

        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;

        let now = Clock::get()?.unix_timestamp;
        let lock_until_ts = if lock_duration_seconds > 0 {
            now.checked_add(lock_duration_seconds)
                .ok_or(VaultError::NumericalOverflow)?
        } else {
            0
        };

        self.vault_config.lock_until_ts = lock_until_ts;
        self.vault_config.spend_limit = spend_limit;
        self.vault_config.period_seconds = spend_period_seconds;
        self.vault_config.period_start_ts = now;
        self.vault_config.withdrawn_this_period = 0;
        self.vault_config.config_bump = bumps.vault_config;

        Ok(())
    }
}
