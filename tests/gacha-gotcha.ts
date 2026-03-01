import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { GachaGotcha } from "../target/types/gacha_gotcha";
import { create, mplCore } from '@metaplex-foundation/mpl-core'
import {
  createGenericFile,
  generateSigner,
  keypairIdentity,
} from '@metaplex-foundation/umi'
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults'
import { irysUploader } from '@metaplex-foundation/umi-uploader-irys'
import { base58 } from '@metaplex-foundation/umi/serializers'
import fs from 'fs';
import { MPL_CORE_PROGRAM_ID } from "@metaplex-foundation/mpl-core";
import { expect } from "chai";
import { Keypair, PublicKey } from "@solana/web3.js";
import wallet from "../../../phantom-wallet/wallet-1/id.json"

describe("gacha-pack", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.GachaGotcha as Program<GachaGotcha>;

  const createNft = async () => {
    const umi = createUmi('https://api.devnet.solana.com')
      .use(mplCore())
      .use(
        irysUploader({
          // mainnet address: "https://node1.irys.xyz"
          // devnet address: "https://devnet.irys.xyz"
          address: 'https://devnet.irys.xyz',
        })
      )
    // Generate a new keypair signer.
    const keypair = umi.eddsa.createKeypairFromSecretKey(
      new Uint8Array(wallet)
    );
    umi.use(keypairIdentity(keypair));

    const imageFile = fs.readFileSync("./image.jpg");

    const umiImageFile = createGenericFile(imageFile, 'image.jpg', {
      tags: [{ name: 'Content-Type', value: 'image/jpg' }],
    });

    console.log('Uploading Image...');
    const imageUri = await umi.uploader.upload([umiImageFile]).catch((err) => {
      throw new Error(err)
    })
    console.log('imageUri: ' + imageUri[0])
    //
    // ** Upload Metadata to Arweave **
    //
    const metadata = {
      name: 'CAT NFT',
      description: 'This is an CAT NFT',
      image: imageUri[0],
      external_url: 'https://example.com',
      attributes: [
        { trait_type: "rarity", value: "Epic" },
      ],
      properties: {
        files: [
          {
            uri: imageUri[0],
            type: 'image/jpeg',
          },
        ],
        category: 'image',
      },
    };

    console.log('Uploading Metadata...')
    const metadataUri = await umi.uploader.uploadJson(metadata).catch((err) => {
      throw new Error(err)
    });
    const asset = generateSigner(umi)
    console.log('Creating NFT...')
    const tx = await create(umi, {
      asset,
      name: 'My NFT',
      uri: metadataUri,
    }).sendAndConfirm(umi)
    // Finally we can deserialize the signature that we can check on chain.
    const signature = base58.deserialize(tx.signature)[0]
    // Log out the signature and the links to the transaction and the NFT.
    console.log('\nNFT Created');
    console.log('View Transaction on Solana Explorer');
    console.log(`https://explorer.solana.com/tx/${signature}?cluster=devnet`);
    console.log('\n');
    console.log('View NFT on Metaplex Explorer');
    console.log(`https://core.metaplex.com/explorer/${asset.publicKey}?cluster=devnet`);

    return asset;  // so tests can use it
  }

  it("lists an MPL Core asset for auction", async () => {
    // 1. Mint Core NFT
    const asset = await createNft();

    const assetPubkey = new PublicKey(asset.publicKey.toString())
    const owner = Keypair.fromSecretKey(new Uint8Array(wallet))
    // 2. Derive auction PDA
    const [auctionPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("auction"), assetPubkey.toBuffer()],
      program.programId
    );

    // 3. Call list instruction
    const minBid = new anchor.BN(0.1 * anchor.web3.LAMPORTS_PER_SOL);
    const duration = new anchor.BN(0); // 0 seconds

    await program.methods
      .list(minBid, duration)
      .accountsStrict({
        seller: owner.publicKey,
        asset: assetPubkey,
        auction: auctionPda,
        systemProgram: anchor.web3.SystemProgram.programId,
        coreProgram: MPL_CORE_PROGRAM_ID,   // from mpl-core
      })
      .signers([owner])                 // seller signs
      .rpc();

    // 4. Fetch and assert auction state
    const auction = await program.account.auction.fetch(auctionPda);

    expect(auction.seller.toBase58()).to.equal(owner.publicKey.toBase58());
    expect(auction.nft.toBase58()).to.equal(assetPubkey.toBase58());
    expect(auction.minimumBid.eq(minBid)).to.be.true;
    expect(auction.active).to.equal(1);


    const bidder = anchor.web3.Keypair.generate();

    const fundTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey: owner.publicKey,
        toPubkey: bidder.publicKey,
        lamports: 2 * anchor.web3.LAMPORTS_PER_SOL,
      })
    );
    await provider.sendAndConfirm(fundTx, [owner]);

    const previousHighestBidder = Keypair.generate();

    // 3. Call list instruction
    const bid = new anchor.BN(0.5 * anchor.web3.LAMPORTS_PER_SOL);

    await program.methods
      .bid(bid)
      .accountsStrict({
        bidder: bidder.publicKey,
        previousHighestBidder: previousHighestBidder.publicKey,
        asset: assetPubkey,
        auction: auctionPda,
        systemProgram: anchor.web3.SystemProgram.programId,
        coreProgram: MPL_CORE_PROGRAM_ID,   // from mpl-core
      })
      .signers([bidder])                 // seller signs
      .rpc();

    // 4. Fetch and assert auction state

    expect(auction.seller.toBase58()).to.equal(owner.publicKey.toBase58());
    expect(auction.nft.toBase58()).to.equal(assetPubkey.toBase58());
    expect(auction.active).to.equal(1);



    const settleTx = await program.methods.settle()
      .accountsStrict({
        seller: owner.publicKey,
        winner: bidder.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        coreProgram: MPL_CORE_PROGRAM_ID,
        auction: auctionPda,
        asset: assetPubkey
      })
      .signers([owner])
      .rpc()

    console.log(`Settle: ${settleTx.toString()}`)
  });

});
