import * as anchor from "@coral-xyz/anchor";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
} from "@solana/web3.js";
import * as sb from "@switchboard-xyz/on-demand";
import { loadSbProgram, setupQueue } from "./utils";

// Local IDL + generated types
import gachaIdl from "../target/idl/gacha_gotcha.json";
import type { GachaGotcha } from "../target/types/gacha_gotcha";

// Your funded wallet (Phantom / CLI JSON)
import walletJson from "../../../phantom-wallet/wallet-1/id.json";

// Program ID from declare_id! in lib.rs
const GACHA_PROGRAM_ID = new PublicKey(
  "6AkHVw9JWm2Ge1S47KmeETiFcnFXBMLr5UMKPQGQm3Wf"
);

// Main keypair used as admin, treasury, and buyer
const MAIN_KP = Keypair.fromSecretKey(Uint8Array.from(walletJson as number[]));

async function loadGachaProgram(
  provider: anchor.Provider
): Promise<anchor.Program<GachaGotcha>> {
  // Ensure IDL has the correct address for Anchor 0.30+ [web:94]
  if (!(gachaIdl as any).address) {
    (gachaIdl as any).address = GACHA_PROGRAM_ID.toBase58();
  }
  return new anchor.Program<GachaGotcha>(gachaIdl as any, provider);
}

async function loadOrCreateRandomnessAccount(
  sbProgram: anchor.Program,
  queue: any
) {
  const rngKp = Keypair.generate();
  const [randomness, createIx] = await sb.Randomness.create(
    sbProgram as any,
    rngKp,
    queue as any
  );
  return { randomness, rngKp, createIx };
}

async function main() {
  console.clear();

  // 0. Provider & wallet (devnet)
  const connection = new Connection("https://api.devnet.solana.com", "confirmed");
  const anchorWallet = new anchor.Wallet(MAIN_KP);
  const provider = new anchor.AnchorProvider(connection, anchorWallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  // 1. Switchboard program + queue (devnet)
  const sbProgram = await loadSbProgram(provider as any);
  const queue = await setupQueue(sbProgram as any);

  // 2. Gacha program from local IDL
  const gachaProgram = await loadGachaProgram(provider);
  console.log(
    "Your Gacha Pack Program:",
    gachaProgram.programId.toString()
  );

  // 3. Load/create randomness account
  const { randomness, rngKp, createIx } = await loadOrCreateRandomnessAccount(
    sbProgram,
    queue
  );

  if (createIx) {
    const createTx = await sb.asV0Tx({
      connection,
      ixs: [createIx],
      payer: MAIN_KP.publicKey,
      signers: [MAIN_KP, rngKp],
      computeUnitPrice: 75_000,
      computeUnitLimitMultiple: 1.3,
    });
    const sig = await connection.sendTransaction(createTx);
    await connection.confirmTransaction(sig, "confirmed");
    console.log("✅ Randomness account created:", sig);
  }

  // 4. Derive pack + rarity PDAs (pack is assumed to already exist)
  const [packPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("pack")],
    gachaProgram.programId
  );
  const [rarityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("rarity"), packPda.toBuffer()],
    gachaProgram.programId
  );
  console.log("pack PDA:", packPda.toString());

  // 5. Buyer = your main wallet
  const buyer = Keypair.generate();

  const fundTx = new anchor.web3.Transaction().add(
    SystemProgram.transfer({
      fromPubkey: MAIN_KP.publicKey,
      toPubkey: buyer.publicKey,
      lamports: 0.2 * 1e9, // 0.2 SOL
    })
  );
  await connection.sendTransaction(fundTx, [MAIN_KP]);
  await new Promise((r) => setTimeout(r, 1000));

  const [userPackPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("userpack"), packPda.toBuffer(), buyer.publicKey.toBuffer()],
    gachaProgram.programId
  );

  // 6. Buy pack
  await gachaProgram.methods
    .buyPack()
    .accountsStrict({
      pack: packPda,
      userPack: userPackPda,
      treasury: MAIN_KP.publicKey,
      buyer: buyer.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([buyer])
    .rpc();
  console.log("✅ Pack bought:", userPackPda.toString());

  // 7. COMMIT PHASE – Switchboard commit + your commit_open in one tx
  const commitIx = await randomness.commitIx(queue as any);
  const commitOpenIx = await gachaProgram.methods
    .commitOpen(rngKp.publicKey) // randomness_account: Pubkey arg
    .accountsStrict({
      buyer: buyer.publicKey,
      pack: packPda,
      userPack: userPackPda,
      randomnessAccount: rngKp.publicKey,
    })
    .signers([buyer])
    .instruction();

  const commitTx = await sb.asV0Tx({
    connection,
    ixs: [commitIx, commitOpenIx],
    payer: MAIN_KP.publicKey,
    signers: [MAIN_KP, buyer],
    computeUnitPrice: 75_000,
    computeUnitLimitMultiple: 1.3,
  });
  const commitSig = await connection.sendTransaction(commitTx);
  await connection.confirmTransaction(commitSig, "confirmed");
  console.log("✅ COMMIT complete:", commitSig);

  // 8. REVEAL PHASE – Switchboard reveal + your open_pack in one tx
  console.log("⏳ Waiting for reveal...");
  await new Promise((r) => setTimeout(r, 3000));

  const revealIx = await randomness.revealIx();
  const openIx = await gachaProgram.methods
    .openPack()
    .accountsStrict({
      userPack: userPackPda,
      pack: packPda,
      rarity: rarityPda,
      randomnessAccount: rngKp.publicKey,
      buyer: buyer.publicKey
    })
    .signers([buyer])
    .instruction();

  console.log("MAIN_KP:", MAIN_KP.publicKey.toBase58());
  console.log("buyer:", buyer.publicKey.toBase58());
  console.log("rngKp:", rngKp.publicKey.toBase58());


  const revealTx = await sb.asV0Tx({
    connection,
    ixs: [revealIx, openIx],
    payer: MAIN_KP.publicKey,
    signers: [MAIN_KP, buyer],
    computeUnitPrice: 75_000,
    computeUnitLimitMultiple: 1.3,
  });
  const revealSig = await connection.sendTransaction(revealTx);
  await connection.confirmTransaction(revealSig, "confirmed");
  console.log("✅ REVEAL complete:", revealSig);

  // 9. Check results
  const userPack = await (gachaProgram.account as any).userPack.fetch(
    userPackPda
  );
  console.log("🎉 PACK REVEALED:", userPack.revealed);
  console.log("Txs: commit=", commitSig, "reveal=", revealSig);


  console.log("Your NFT mints:", userPack.nftMints.map(mint => mint.toBase58()));
  // ["7xK8pAbCdEfGhIjKlMnOpQrStUvWxYzA1bCdEf2", "9aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890AbCd", ...]
}

main().catch(console.error);
