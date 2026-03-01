# Capstone Vault Constraint Logic

The program enforces constraints in two layers:

1. **Anchor account constraints** (`#[account(...)]`)  
   These are checked before instruction logic runs.
2. **Runtime guards** (`require!(...)`)  
   These enforce business rules that depend on runtime values.

## PDA and Seed Model

The program uses fixed seed prefixes from `src/constants.rs`:

- `STATE_SEED = "state"`: user state PDA namespace
- `VAULT_SEED = "vault"`: vault PDA namespace
- `CONFIG_SEED = "config"`: restricted-config PDA namespace

Per user:

- `vault_state` PDA: `[STATE_SEED, user_pubkey]`
- `vault` PDA: `[VAULT_SEED, vault_state_pubkey]`
- `vault_config` PDA (restricted mode): `[CONFIG_SEED, user_pubkey]`

This structure guarantees deterministic ownership mapping:

- Only the correct user's state/config PDAs are accepted
- The vault account is tied to the exact `vault_state`
- Signer seeds can only be derived from valid stored bumps

## Shared Constraint Patterns

These patterns appear in multiple instructions:

- `user: Signer`: caller must sign the transaction
- `seeds + bump`: incoming accounts must match expected PDA derivations
- `mut`: account is writable where state or lamports change
- `close = user`: account rent is reclaimed by `user` on close

These checks prevent account substitution attacks (passing someone else's PDA), unauthorized writes, and rent leakage.

## Instruction-by-Instruction Rules

## `initialize`

Account constraints:

- `vault_state` is **created** with:
  - payer = `user`
  - seeds = `[STATE_SEED, user]`
  - allocated space for `VaultState`
- `vault` must be the PDA derived from `[VAULT_SEED, vault_state]`

Runtime logic:

- computes required rent exemption for the vault account data length
- transfers that rent-exempt minimum from `user` to `vault`
- stores the bumps in `vault_state` for future signer derivation

Security intent:

- ensures vault PDA starts rent-safe
- pins future authority to saved bump values

## `initialize_restricted`

Account constraints:

- same `vault_state` and `vault` rules as `initialize`
- creates `vault_config` at `[CONFIG_SEED, user]`

Runtime guards:

- `lock_duration_seconds >= 0`  
  Prevents nonsensical negative lock time.
- if `spend_limit > 0`, then `spend_period_seconds > 0`  
  A limit must have a real period window.
- if `spend_limit == 0`, then `spend_period_seconds == 0`  
  Prevents partial/ambiguous config.
- `checked_add` for lock timestamp  
  Prevents integer overflow.

Runtime initialization:

- funds vault rent exemption
- saves state/config bumps
- stores lock/period/limit fields and resets period withdrawal counter

Security intent:

- ensures restricted mode starts in a self-consistent configuration
- blocks overflow-based bypasses

## `deposit`

Account constraints:

- vault PDA must match `vault_state`
- `vault_state` PDA must match calling user

Runtime logic:

- transfers `amount` lamports from `user` to `vault`

Security intent:

- caller can only deposit into their own mapped vault context
- prevents directing funds to an arbitrary PDA via account substitution

## `withdraw`

Account constraints:

- same user-state-vault linkage as `deposit`

Runtime guard:

- `vault.lamports() >= amount`  
  Prevents underfunded withdrawal attempts.

Signer derivation:

- program signs transfer from vault using seeds:
  `[VAULT_SEED, vault_state_pubkey, vault_bump]`

Security intent:

- only the canonical vault PDA can authorize payout
- avoids accidental or malicious overdraft attempts

## `withdraw_restricted`

Account constraints:

- same constraints as `withdraw`
- plus `vault_config` PDA tied to caller user

Runtime guards:

- sufficient vault funds
- if time lock is enabled (`lock_until_ts > 0`), current time must be past lock
- if spend limit is enabled:
  - period must be positive
  - period end computation uses `checked_add` (overflow-safe)
  - period auto-resets when expired
  - `withdrawn_this_period + amount` uses `checked_add`
  - new total must remain `<= spend_limit`

State updates:

- updates `period_start_ts` and `withdrawn_this_period` when period rolls
- persists updated withdrawal counter after successful transfer

Security intent:

- enforces lock and rate-limiting policy on-chain
- keeps period accounting consistent and overflow-safe

## `close`

Account constraints:

- user-linked `vault_state`
- `close = user` on `vault_state`
- vault PDA derived from that state

Runtime logic:

- transfers all vault lamports back to `user`
- closes `vault_state` (rent returned to user by Anchor)

Security intent:

- allows clean teardown while preserving ownership boundaries

## `close_restricted`

Account constraints:

- same as `close`
- also requires user-linked `vault_config` with `close = user`

Runtime logic:

- transfers full vault balance to user
- closes both `vault_state` and `vault_config`

Security intent:

- prevents leftover config/state accounts after restricted vault teardown

## Error Semantics

`src/error.rs` defines explicit failure reasons:

- `VaultStillLocked`: attempted withdrawal before timelock expires
- `SpendLimitExceeded`: period spending exceeds configured cap
- `PeriodRequiredForSpendLimit`: limit set without valid positive period
- `InvalidLockDuration`: negative lock duration passed
- `InvalidSpendLimitConfig`: inconsistent limit/period setup
- `InsufficientVaultFunds`: requested more than vault balance
- `NumericalOverflow`: safe arithmetic overflow detected

These errors make policy failures deterministic and easier to debug from clients/tests.
