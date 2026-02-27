import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import * as sb from "@switchboard-xyz/on-demand";

/**
 * Load Switchboard On-Demand program using the given Anchor provider.
 */
export async function loadSbProgram(
  provider: anchor.Provider
): Promise<anchor.Program> {
  const sbProgramId = await sb.getProgramId(provider.connection);
  const sbIdl = await anchor.Program.fetchIdl(sbProgramId, provider);
  if (!sbIdl) {
    throw new Error(`Switchboard IDL not found for ${sbProgramId.toBase58()}`);
  }

  // Ensure the IDL has the program address set
  if (!("address" in sbIdl) || !sbIdl.address) {
    (sbIdl as any).address = sbProgramId.toBase58();
  }

  // For Anchor 0.30+ the correct ctor is (idl, provider)
  return new anchor.Program(sbIdl as anchor.Idl, provider);
}

/**
 * Get the default randomness queue for the current endpoint.
 */
export async function setupQueue(program: anchor.Program): Promise<PublicKey> {
  // Type cast to avoid anchor 0.31 vs 0.32 type mismatch
  const queueAccount = await sb.getDefaultQueue(
    (program as any).provider.connection.rpcEndpoint
  );
  console.log("Queue account", queueAccount.pubkey.toString());

  try {
    await queueAccount.loadData();
  } catch (err) {
    console.error("Queue not found, ensure you are using devnet in your env");
    process.exit(1);
  }

  return queueAccount.pubkey;
}
