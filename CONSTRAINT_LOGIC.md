# Capstone Vault

## Overview

The program has two modes. Each user chooses one at initialization:

- **Basic mode** — simple deposit/withdraw with no restrictions.
- **Restricted mode** — adds an optional time lock (no withdrawals until a set time) and an optional spend limit (max SOL withdrawable per rolling period).

Constraints are enforced in two layers:

1. **Anchor account constraints** (`#[account(...)]`) — verified before the instruction runs (correct PDA, correct owner, writable, etc.)
2. **Runtime guards** (`require!(...)`) — business rules checked inside the instruction (lock expired? spend limit OK? enough funds?)

---

## Accounts (PDAs)

All accounts are derived from the user's public key, so each user has their own isolated set:

| Account        | Seeds                    | Purpose                                        |
| -------------- | ------------------------ | ---------------------------------------------- |
| `vault_state`  | `["state", user]`        | Stores PDA bumps                               |
| `vault`        | `["vault", vault_state]` | Holds the SOL                                  |
| `vault_config` | `["config", user]`       | Stores lock/spend rules (restricted mode only) |

---

## Instructions

### Basic Mode

**`initialize`**

- Creates `vault_state` and funds `vault` with the rent-exempt minimum.

**`deposit`**

- Transfers SOL from user to vault. No restrictions.

**`withdraw`**

- Transfers SOL from vault to user. Requires vault has enough funds.

**`close`**

- Drains all vault SOL back to user and closes `vault_state` (rent returned).

### Restricted Mode

**`initialize_restricted`**

- Same as `initialize`, but also creates `vault_config` with user-defined rules:
  - `lock_duration_seconds` — how long until withdrawals are allowed (0 = no lock)
  - `spend_limit` — max lamports withdrawable per period (0 = no limit)
  - `spend_period_seconds` — rolling window for the spend limit (must be > 0 if limit is set)

**`deposit`** — same as basic mode.

**`withdraw_restricted`**

- Checks in order:
  1. Vault has sufficient funds.
  2. Time lock has expired (if set).
  3. Withdrawal stays within the spend limit for the current period (if set). Period auto-resets when expired.

**`close_restricted`**

- Same as `close`, but also closes `vault_config`.

---

## Errors

| Error                         | Cause                                         |
| ----------------------------- | --------------------------------------------- |
| `VaultStillLocked`            | Withdrawal attempted before time lock expires |
| `SpendLimitExceeded`          | Withdrawal would exceed the period cap        |
| `PeriodRequiredForSpendLimit` | Spend limit set but period is 0               |
| `InvalidLockDuration`         | Negative lock duration passed                 |
| `InvalidSpendLimitConfig`     | Period set but spend limit is 0               |
| `InsufficientVaultFunds`      | Requested more than vault balance             |
| `NumericalOverflow`           | Safe arithmetic overflow detected             |
