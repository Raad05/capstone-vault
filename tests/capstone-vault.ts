import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { CapstoneVault } from "../target/types/capstone_vault";

describe("capstone_vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CapstoneVault as Program<CapstoneVault>;
  const user = provider.wallet.publicKey;

  const [vaultStatePda, stateBump] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.toBuffer()],
      program.programId,
    );

  const [vaultPda, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), vaultStatePda.toBuffer()],
    program.programId,
  );

  before(async () => {
    await provider.connection.requestAirdrop(
      user,
      10 * anchor.web3.LAMPORTS_PER_SOL,
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));
  });

  it("Initialize the vault", async () => {
    await program.methods
      .initialize()
      .accountsStrict({
        user: user,
        vaultState: vaultStatePda,
        vault: vaultPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const vaultState = await program.account.vaultState.fetch(vaultStatePda);
    expect(vaultState.vaultBump).to.equal(vaultBump);
    expect(vaultState.stateBump).to.equal(stateBump);

    const vaultBalance = await provider.connection.getBalance(vaultPda);
    const rentExempt =
      await provider.connection.getMinimumBalanceForRentExemption(0);
    expect(vaultBalance).to.equal(rentExempt);
  });

  it("Deposit SOL into the vault", async () => {
    const depositAmount = 1 * anchor.web3.LAMPORTS_PER_SOL;

    const initialVaultBalance = await provider.connection.getBalance(vaultPda);
    const initialUserBalance = await provider.connection.getBalance(user);

    await program.methods
      .deposit(new anchor.BN(depositAmount))
      .accountsStrict({
        user: user,
        vault: vaultPda,
        vaultState: vaultStatePda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const finalVaultBalance = await provider.connection.getBalance(vaultPda);
    const finalUserBalance = await provider.connection.getBalance(user);

    expect(finalVaultBalance).to.equal(initialVaultBalance + depositAmount);
    expect(finalUserBalance).to.equal(
      initialUserBalance - depositAmount - 5000,
    );
  });

  it("Withdraw SOL from the vault", async () => {
    const withdrawAmount = 0.5 * anchor.web3.LAMPORTS_PER_SOL;

    const initialVaultBalance = await provider.connection.getBalance(vaultPda);
    const initialUserBalance = await provider.connection.getBalance(user);

    await program.methods
      .withdraw(new anchor.BN(withdrawAmount))
      .accountsStrict({
        user: user,
        vault: vaultPda,
        vaultState: vaultStatePda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const finalVaultBalance = await provider.connection.getBalance(vaultPda);
    const finalUserBalance = await provider.connection.getBalance(user);

    expect(finalVaultBalance).to.equal(initialVaultBalance - withdrawAmount);
    expect(finalUserBalance).to.equal(
      initialUserBalance + withdrawAmount - 5000,
    );
  });

  it("Close the vault", async () => {
    const initialVaultBalance = await provider.connection.getBalance(vaultPda);
    const initialVaultStateBalance = await provider.connection.getBalance(
      vaultStatePda,
    );
    const initialUserBalance = await provider.connection.getBalance(user);

    await program.methods
      .close()
      .accountsStrict({
        user: user,
        vault: vaultPda,
        vaultState: vaultStatePda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const finalUserBalance = await provider.connection.getBalance(user);

    expect(await provider.connection.getBalance(vaultPda)).to.equal(0);
    expect(await provider.connection.getAccountInfo(vaultStatePda)).to.be.null;
    expect(finalUserBalance).to.equal(
      initialUserBalance +
        initialVaultBalance +
        initialVaultStateBalance -
        5000,
    );
  });
});

describe("capstone_vault_restricted", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CapstoneVault as Program<CapstoneVault>;

  const restrictedUser = anchor.web3.Keypair.generate();
  const oneSol = anchor.web3.LAMPORTS_PER_SOL;
  const lockDurationSeconds = 2;
  const spendLimit = 1 * oneSol;
  const spendPeriodSeconds = 3;

  const [restrictedVaultStatePda] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("state"), restrictedUser.publicKey.toBuffer()],
      program.programId,
    );

  const [restrictedVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), restrictedVaultStatePda.toBuffer()],
    program.programId,
  );

  const [restrictedVaultConfigPda] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("config"), restrictedUser.publicKey.toBuffer()],
      program.programId,
    );

  before(async () => {
    await provider.connection.requestAirdrop(
      restrictedUser.publicKey,
      10 * oneSol,
    );
    await new Promise((resolve) => setTimeout(resolve, 1000));
  });

  it("Initialize restricted vault", async () => {
    await program.methods
      .initializeRestricted(
        new anchor.BN(lockDurationSeconds),
        new anchor.BN(spendLimit),
        new anchor.BN(spendPeriodSeconds),
      )
      .accountsStrict({
        user: restrictedUser.publicKey,
        vaultState: restrictedVaultStatePda,
        vault: restrictedVaultPda,
        vaultConfig: restrictedVaultConfigPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([restrictedUser])
      .rpc();

    const vaultConfig = await program.account.vaultConfig.fetch(
      restrictedVaultConfigPda,
    );
    expect(vaultConfig.spendLimit.toNumber()).to.equal(spendLimit);
    expect(vaultConfig.periodSeconds.toNumber()).to.equal(spendPeriodSeconds);
    expect(vaultConfig.withdrawnThisPeriod.toNumber()).to.equal(0);
  });

  it("Deposit into restricted vault", async () => {
    const depositAmount = 2 * oneSol;
    const initialBalance = await provider.connection.getBalance(
      restrictedVaultPda,
    );

    await program.methods
      .deposit(new anchor.BN(depositAmount))
      .accountsStrict({
        user: restrictedUser.publicKey,
        vault: restrictedVaultPda,
        vaultState: restrictedVaultStatePda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([restrictedUser])
      .rpc();

    const finalBalance = await provider.connection.getBalance(
      restrictedVaultPda,
    );
    expect(finalBalance).to.equal(initialBalance + depositAmount);
  });

  it("Rejects withdrawal before time lock expires", async () => {
    let failed = false;
    try {
      await program.methods
        .withdrawRestricted(new anchor.BN(0.5 * oneSol))
        .accountsStrict({
          user: restrictedUser.publicKey,
          vault: restrictedVaultPda,
          vaultState: restrictedVaultStatePda,
          vaultConfig: restrictedVaultConfigPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([restrictedUser])
        .rpc();
    } catch {
      failed = true;
    }
    expect(failed).to.equal(true);
  });

  it("Allows withdrawal after time lock expires, rejects when spend limit exceeded", async () => {
    await new Promise((resolve) =>
      setTimeout(resolve, (lockDurationSeconds + 1) * 1000),
    );

    // First withdrawal — within limit
    await program.methods
      .withdrawRestricted(new anchor.BN(0.6 * oneSol))
      .accountsStrict({
        user: restrictedUser.publicKey,
        vault: restrictedVaultPda,
        vaultState: restrictedVaultStatePda,
        vaultConfig: restrictedVaultConfigPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([restrictedUser])
      .rpc();

    // Second withdrawal — would exceed 1 SOL limit (0.6 + 0.5 = 1.1)
    let failedSpendLimit = false;
    try {
      await program.methods
        .withdrawRestricted(new anchor.BN(0.5 * oneSol))
        .accountsStrict({
          user: restrictedUser.publicKey,
          vault: restrictedVaultPda,
          vaultState: restrictedVaultStatePda,
          vaultConfig: restrictedVaultConfigPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([restrictedUser])
        .rpc();
    } catch {
      failedSpendLimit = true;
    }
    expect(failedSpendLimit).to.equal(true);
  });

  it("Allows withdrawal after spend period resets", async () => {
    await new Promise((resolve) =>
      setTimeout(resolve, (spendPeriodSeconds + 1) * 1000),
    );

    const initialBalance = await provider.connection.getBalance(
      restrictedVaultPda,
    );
    const withdrawAmount = 0.5 * oneSol;

    await program.methods
      .withdrawRestricted(new anchor.BN(withdrawAmount))
      .accountsStrict({
        user: restrictedUser.publicKey,
        vault: restrictedVaultPda,
        vaultState: restrictedVaultStatePda,
        vaultConfig: restrictedVaultConfigPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([restrictedUser])
      .rpc();

    const finalBalance = await provider.connection.getBalance(
      restrictedVaultPda,
    );
    expect(finalBalance).to.equal(initialBalance - withdrawAmount);
  });

  it("Close restricted vault", async () => {
    const initialUserBalance = await provider.connection.getBalance(
      restrictedUser.publicKey,
    );
    const initialVaultBalance = await provider.connection.getBalance(
      restrictedVaultPda,
    );

    await program.methods
      .closeRestricted()
      .accountsStrict({
        user: restrictedUser.publicKey,
        vault: restrictedVaultPda,
        vaultState: restrictedVaultStatePda,
        vaultConfig: restrictedVaultConfigPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([restrictedUser])
      .rpc();

    expect(await provider.connection.getBalance(restrictedVaultPda)).to.equal(
      0,
    );
    expect(await provider.connection.getAccountInfo(restrictedVaultStatePda)).to
      .be.null;
    expect(await provider.connection.getAccountInfo(restrictedVaultConfigPda))
      .to.be.null;

    const finalUserBalance = await provider.connection.getBalance(
      restrictedUser.publicKey,
    );
    expect(finalUserBalance).to.be.greaterThan(
      initialUserBalance + initialVaultBalance - 5000,
    );
  });
});
