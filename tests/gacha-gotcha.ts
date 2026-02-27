import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { GachaGotcha } from "../target/types/gacha_gotcha";
import { expect } from "chai";
import * as sb from "@switchboard-xyz/on-demand";
import { Keypair } from "@solana/web3.js";
import { RandomnessService } from "@switchboard-xyz/solana-randomness-service";

describe("gacha-pack", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.GachaGotcha as Program<GachaGotcha>;

  before(async () => {

    const { keypair, connection, program } = await sb.AnchorUtils.loadEnv();

    const queue = await sb.getDefaultQueue(connection.rpcEndpoint);

    const sbProgramId = await sb.getProgramId(provider.connection);
    const sbIdl = await anchor.Program.fetchIdl(sbProgramId, provider);
    const sbProgram = new anchor.Program(sbIdl!, provider);
  })

  it("Creates pack config", async () => {
    const [pack] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pack")],
      program.programId
    );
    const [rarity] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rarity"), pack.toBuffer()],
      program.programId
    );

    await program.methods
      .createPack()
      .accountsStrict({
        pack,
        rarity,
        admin: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();  // ← No signers needed!

    const packAccount = await program.account.packConfig.fetch(pack);
    console.log("Pack:", packAccount);
    expect(packAccount.packSize).to.equal(5);
  });

  it("User buys a pack", async () => {
    const buyer = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(buyer.publicKey, 2e9);
    await new Promise(r => setTimeout(r, 2000));

    const [pack] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pack")],
      program.programId
    );
    const [userPack] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("userpack"), pack.toBuffer(), buyer.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .buyPack()
      .accountsStrict({
        pack,
        userPack,
        treasury: provider.wallet.publicKey,  // Admin wallet
        buyer: buyer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    const fetchedPack = await program.account.packConfig.fetch(pack);
    expect(fetchedPack.soldCount).to.equal(1);
  });


  it("Full pack flow: buy → commit → open", async () => {

    const { keypair, connection, program } = await sb.AnchorUtils.loadEnv();

    const queue = await sb.getDefaultQueue(connection.rpcEndpoint);

    const sbProgramId = await sb.getProgramId(provider.connection);
    const sbIdl = await anchor.Program.fetchIdl(sbProgramId, provider);
    const sbProgram = new anchor.Program(sbIdl!, provider);
    // Generate keypair for randomness account
    const rngKp = Keypair.generate();

    // Create the randomness account
    const [randomness, createIx] = await sb.Randomness.create(sbProgram, rngKp, queue);

    // Send creation transaction
    const createTx = await sb.asV0Tx({
      connection,
      ixs: [createIx],
      payer: keypair.publicKey,
      signers: [keypair, rngKp],
      computeUnitPrice: 75_000,
      computeUnitLimitMultiple: 1.3,
    });

    await connection.sendTransaction(createTx);

    const requestKeypair = anchor.web3.Keypair.generate();
    const [packPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pack")],
      program.programId
    );
    const [rarityPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("rarity"), packPda.toBuffer()],
      program.programId
    );

    const buyer = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(buyer.publicKey, 2e9);
    await new Promise(r => setTimeout(r, 2000));

    // 1. BUY PACK FIRST (creates user_pack)
    const [userPack] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("userpack"), packPda.toBuffer(), buyer.publicKey.toBuffer()],
      program.programId
    );

    const txSig = await program.methods
      .commitOpen()  // Your instruction that CPI's to randomness service
      .accountsStrict({
        buyer: buyer.publicKey
        pack: packPda,
        randomness: 
  })
      .signers([requestKeypair])
      .rpc();

    await program.methods
      .buyPack()
      .accountsStrict({
        pack: packPda,
        userPack,
        treasury: provider.wallet.publicKey,
        buyer: buyer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([buyer])
      .rpc();

    // 2. COMMIT
    await program.methods
      .commitOpen()
      .accountsStrict({
        buyer: buyer.publicKey,
        pack: packPda,
        userPack,
        randomness: anchor.web3.Keypair.generate().publicKey, // dummy
      })
      .signers([buyer])
      .rpc();

    let fetched = await program.account.userPack.fetch(userPack);
    expect(fetched.status).to.deep.equal({ committed: {} });

  });
});

export async function loadSbProgram(
  provider: anchor.Provider
): Promise<anchor.Program> {
  const sbProgramId = await sb.getProgramId(provider.connection);
  const sbIdl = await anchor.Program.fetchIdl(sbProgramId, provider);
  const sbProgram = new anchor.Program(sbIdl!, provider);
  return sbProgram;
}



export async function myAnchorProgram(
  provider: anchor.Provider,
  keypath: string
): Promise<anchor.Program> {
  const myProgramKeypair = await sb.AnchorUtils.initKeypairFromFile(keypath);
  const pid = myProgramKeypair.publicKey;
  const idl = (await anchor.Program.fetchIdl(pid, provider))!;
  if (idl == null) {
    console.error("IDL not found for the program at", pid.toString());
    process.exit(1);
  }
  if (idl?.address == undefined || idl?.address == null) {
    idl.address = pid.toString();
  }
  const program = new anchor.Program(idl, provider);
  return program;
}
