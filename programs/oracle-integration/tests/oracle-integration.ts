import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { OracleIntegration } from "../target/types/oracle_integration";
import { expect } from "chai";

describe("oracle-integration", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.OracleIntegration as Program<OracleIntegration>;
  const provider = anchor.AnchorProvider.env();

  it("Initializes oracle configuration", async () => {
    const oracleConfig = anchor.web3.Keypair.generate();
    const pythFeed = new anchor.web3.PublicKey("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD"); // BTC/USD Pyth feed
    const switchboardFeed = new anchor.web3.PublicKey("8SXvChNYFhRq4EZuZvnhjrB3jJRQCv4k3P4W6hesH3Ee"); // Example Switchboard feed

    const tx = await program.methods
      .initializeOracle()
      .accounts({
        oracleConfig: oracleConfig.publicKey,
        pythFeed: pythFeed,
        switchboardFeed: switchboardFeed,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([oracleConfig])
      .rpc();

    console.log("Oracle initialized with tx signature:", tx);

    const oracleConfigAccount = await program.account.oracleConfig.fetch(oracleConfig.publicKey);
    expect(oracleConfigAccount.authority.toString()).to.equal(provider.wallet.publicKey.toString());
    expect(oracleConfigAccount.maxPriceDeviation).to.equal(500);
    expect(oracleConfigAccount.maxPriceAge.toNumber()).to.equal(60);
  });

  it("Fetches and aggregates prices from multiple oracles", async () => {
    const oracleConfig = anchor.web3.Keypair.generate();
    const priceFeed = anchor.web3.Keypair.generate();
    const pythFeed = new anchor.web3.PublicKey("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD");
    const switchboardFeed = new anchor.web3.PublicKey("8SXvChNYFhRq4EZuZvnhjrB3jJRQCv4k3P4W6hesH3Ee");

    // First initialize the oracle
    await program.methods
      .initializeOracle()
      .accounts({
        oracleConfig: oracleConfig.publicKey,
        pythFeed: pythFeed,
        switchboardFeed: switchboardFeed,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([oracleConfig])
      .rpc();

    // Then fetch aggregated price
    const tx = await program.methods
      .fetchAggregatedPrice()
      .accounts({
        oracleConfig: oracleConfig.publicKey,
        priceFeed: priceFeed.publicKey,
        pythFeed: pythFeed,
        switchboardFeed: switchboardFeed,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([priceFeed])
      .rpc();

    console.log("Price aggregation tx signature:", tx);

    const priceFeedAccount = await program.account.priceFeed.fetch(priceFeed.publicKey);
    expect(priceFeedAccount.markPrice.toNumber()).to.be.greaterThan(0);
    expect(priceFeedAccount.lastUpdated.toNumber()).to.be.greaterThan(0);
  });

  it("Handles price deviation validation", async () => {
    // This test would require mock oracles with controlled price differences
    // Implementation would depend on specific test setup
  });

  it("Validates price staleness", async () => {
    // This test would require mock oracles with controlled timestamps
    // Implementation would depend on specific test setup
  });
});
