use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

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
        seeds = [b"state", user.key().as_ref()], 
        bump,
        space = VaultState::DISCRIMINATOR.len() + VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        // Get the amount of lamports needed to make the vault rent exempt
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());

        // Transfer the rent-exempt amount from the user to the vault
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
        seeds = [b"state", user.key().as_ref()],
        bump,
        space = VaultState::DISCRIMINATOR.len() + VaultState::INIT_SPACE,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        init,
        payer = user,
        seeds = [b"config", user.key().as_ref()],
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

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()], 
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.system_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(seeds = [b"state", user.key().as_ref()], bump = vault_state.state_bump,)]
    pub vault_state: Account<'info, VaultState>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        require!(
            self.vault.lamports() >= amount,
            VaultError::InsufficientVaultFunds
        );

        let cpi_program = self.system_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct WithdrawRestricted<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(seeds = [b"state", user.key().as_ref()], bump = vault_state.state_bump,)]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [b"config", user.key().as_ref()],
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
            b"vault",
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

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
        close = user,
    )]
    pub vault_state: Account<'info, VaultState>,
    pub system_program: Program<'info, System>,
}

impl<'info> Close<'info> {
    pub fn close(&mut self) -> Result<()> {
        let cpi_program = self.system_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        let amount = self.vault.lamports();
        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CloseRestricted<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
        close = user,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [b"config", user.key().as_ref()],
        bump = vault_config.config_bump,
        close = user,
    )]
    pub vault_config: Account<'info, VaultConfig>,
    pub system_program: Program<'info, System>,
}

impl<'info> CloseRestricted<'info> {
    pub fn close_restricted(&mut self) -> Result<()> {
        let cpi_program = self.system_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        let amount = self.vault.lamports();
        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

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
