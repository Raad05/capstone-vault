use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{
    constants::{CONFIG_SEED, STATE_SEED, VAULT_SEED},
    state::{VaultConfig, VaultState},
};

#[derive(Accounts)]
pub struct CloseRestricted<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = vault_state.state_bump,
        close = user,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [CONFIG_SEED, user.key().as_ref()],
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
            VAULT_SEED,
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
