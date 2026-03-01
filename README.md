# Gacha Gotcha – Turbin3 Capstone

On-chain **gacha packs + NFT marketplace** built on Solana using Anchor and Metaplex Core.

Users can:

- Buy mystery packs.
- Open them with verifiable randomness (via Switchboard on-demand).
- Receive NFT “cards”.
- List a Core NFT in an on-chain auction.
- Bid, settle, or cancel the auction (if no bids yet). [file:1]

---

## What this project does

At a high level:

- **Packs & randomness**
  - Admins create “packs” that define rarities and rewards.
  - Users buy packs by paying SOL into the program.
  - Opening a pack uses a randomness flow wired for **Switchboard On-Demand VRF** so card outcomes can be provably fair. (The project is structured so this can run on devnet, but the listing/auction logic is tested independently of Switchboard.) [file:1]

- **NFT rewards**
  - When a pack is opened, the program records which NFT “cards” the user drew in a per-pack `UserPack` account.
  - For the marketplace demo, tests mint **real Metaplex Core NFTs** on devnet and pass those asset pubkeys down into the auction instructions. [file:1]

- **Marketplace / auction**
  - Any Core NFT owned by the seller can be listed as an auction.
  - Bidders send SOL to an auction PDA that acts as the SOL vault.
  - The highest bidder wins and either the seller settles or cancels (with rules described below). [file:1]

---

## On-chain accounts (high level)

- **Pack config (`Pack` / `PackConfig` – naming depends on your version)**
  - Stores metadata about a pack: price, number of cards, and rarity distribution.

- **User pack (`UserPack`)**
  - One per “pack purchase”.
  - Fields include:
    - `owner`: user wallet.
    - `revealed`: which rarities were drawn.
    - `nft_mints`: array of NFT pubkeys (or deterministic IDs) for that pack. [file:1]

- **Auction (`Auction`)**
  - PDA derived from `[b"auction", asset_pubkey.as_ref()]`.
  - Fields include:
    - `seller`: original owner of the NFT.
    - `nft`: Core asset pubkey.
    - `highest_bidder`: current leading bidder.
    - `highest_bid`: highest bid in lamports.
    - `minimum_bid`: minimum starting price.
    - `end_time`: unix timestamp when auction is allowed to settle.
    - `bump`: PDA bump.
    - `active`: 1 for active, 0 for ended/cancelled. [file:1]

The **auction account’s lamports** are used as the SOL vault for bids and payouts; no separate WSOL or token vault is needed. [file:1]

---

## Randomness with Switchboard (on-demand)

The pack-opening flow is designed around **Switchboard On-Demand VRF**:

- A **commit step** requests randomness from Switchboard and writes request state on-chain.
- An **open step** consumes the VRF result to decide rarities/cards and writes results into `UserPack`. [file:1]
- For this README, it’s enough to know:
  - The program is structured to support verifiable randomness.
  - The marketplace / auction tests run independently of Switchboard so they can run cleanly on devnet.

You can keep the VRF wiring as a separate “chunk” and still test listing/bidding/settling using deterministic or mocked randomness during development. [file:1]

---

## Instructions

### 1. `create_pack`

**Who calls it:** Admin only.

**What it does:**

- Creates and initializes a new `Pack` / `PackConfig` PDA, usually with seeds that include an admin key + a numeric pack ID or nonce.
- Stores:
  - Price per pack in lamports.
  - Number of cards per pack.
  - Rarity probabilities / configuration. [file:1]

**Notes:**

- Using a `pack_id` or admin + nonce in the seeds allows **multiple different packs** instead of a single global pack. [file:1]

---

### 2. `buy_pack`

**Who calls it:** Any user.

**What it does:**

- User pays the pack price in SOL to the program.
- Program:
  - Derives a `UserPack` PDA (e.g. seeds like `[b"userpack", pack.key.as_ref(), buyer.key.as_ref(), ...]`).
  - Initializes `UserPack` for that specific user and pack.
  - Records:
    - `owner` = buyer.
    - `pack` reference.
    - “Not opened” status. [file:1]

This is the on-chain record that the user owns a specific unopened pack.

---

### 3. `commit_open` (randomness commit)

**Who calls it:** Pack owner (the user).

**What it does:**

- Starts the randomness flow for opening a pack.
- Typically:
  - Validates that this `UserPack` is unopened and owned by the signer.
  - Interacts with **Switchboard On-Demand VRF** to request randomness and stores the request ID / state on-chain.
- Marks the pack as “waiting for randomness”. [file:1]

You can think of it as “locking in” the decision to open and preventing the outcome from being manipulated later.

---

### 4. `open_pack` (reveal)

**Who calls it:** Pack owner (or a Switchboard callback, depending on your VRF design).

**What it does:**

- Confirms that randomness is ready (Switchboard callback or stored result).
- Uses that randomness to:
  - Sample rarities according to the pack’s configuration.
  - Choose specific NFT mints (or deterministic IDs) for each card.
- Writes results into `UserPack`:
  - `revealed` rarities.
  - `nft_mints` array (one per card). [file:1]

After this, `UserPack` is the **canonical on-chain record** of which NFTs that user owns from that pack. The frontend can query all `UserPack` accounts owned by a wallet to display the full “inventory” of cards. [file:1]

---

### 5. `list` (list Core NFT for auction)

**Who calls it:** Seller (Core NFT owner).

**Accounts (simplified):**

- `seller: Signer`
- `asset: UncheckedAccount` (MPL Core asset; checked by owner = Core program and non-empty data)
- `auction: Account<Auction>` (PDA, seeds `[b"auction", asset.key().as_ref()]`)
- `system_program`
- `core_program` (MPL Core program ID) [file:1]

**What it does:**

- Validates:
  - `seller` is the current owner of the Core asset.
  - `asset` is a valid Core asset account (owned by `CORE_PROGRAM_ID`).
- Initializes an `Auction` PDA:
  - Writes seller, asset pubkey, minimum bid, end time, bump, active=1, etc.
- Optionally logs a helpful message like:
  - “NFT `<asset_key>` is live on the marketplace with min bid X lamports.” [file:1]
- Transfers the Core asset’s ownership from `seller` to the `auction` PDA via `TransferV1CpiBuilder`, so the program holds the NFT in escrow during the auction. [file:1]

---

### 6. `bid`

**Who calls it:** Any bidder with enough SOL.

**Accounts (simplified):**

- `bidder: Signer`
- `asset: UncheckedAccount` (same Core asset)
- `auction: Account<Auction>` (PDA)
- `previous_highest_bidder: UncheckedAccount` (used only when outbidding)
- `system_program`
- `core_program` (Core program, passed through) [file:1]

**What it does:**

- Checks:
  - Auction is active.
  - `bid >= minimum_bid`.
  - `bid > current highest_bid`. [file:1]
- Transfers SOL:
  - Uses a SystemProgram CPI transfer from `bidder` → `auction` account.
  - This makes `auction`’s lamports the escrowed SOL vault.
- Refunds previous highest bidder (if any):
  - When `auction.highest_bid > 0`, requires that `previous_highest_bidder.key == auction.highest_bidder`.
  - Moves lamports directly:
    - `auction.lamports -= old_highest_bid`.
    - `previous_highest_bidder.lamports += old_highest_bid`.
- Updates state:
  - `auction.highest_bid = bid`.
  - `auction.highest_bidder = bidder.key()`. [file:1]

---

### 7. `settle`

**Who calls it:** Seller (auction creator), after the auction ends.

**Accounts (simplified):**

- `seller: Signer`
- `asset: UncheckedAccount` (Core asset)
- `winner: SystemAccount` (must equal `auction.highest_bidder`)
- `auction: Account<Auction>` (PDA, `close = seller` in your final version)
- `system_program`
- `core_program` (MPL Core) [file:1]

**What it does:**

- Checks:
  - Auction is active.
  - Current time ≥ `auction.end_time`.
  - There is at least one bid (`highest_bid > 0`).
  - `seller` matches `auction.seller`.
  - `winner` matches `auction.highest_bidder`. [file:1]
- Pays seller from auction’s SOL vault:
  - Moves lamports directly:
    - `auction.lamports -= highest_bid`.
    - `seller.lamports += highest_bid`.
- Transfers the Core asset from `auction` → `winner`:
  - Uses `TransferV1CpiBuilder` with `auction` PDA as authority and `invoke_signed` with the same seeds as `list`.
- Closes or deactivates the auction:
  - In your latest version, `auction` is closed to the seller or marked `active = 0`, depending on the instruction definition. [file:1]

---

### 8. `cancel`

**Who calls it:** Seller.

**Important design choice:**  
`cancel` is allowed **only if there are no bids yet**. If `highest_bid > 0`, it fails with `BidTooLow` (used here as a generic “cannot cancel once a bid exists” error). [file:1]

**Accounts (final version):**

- `seller: Signer`
- `asset: UncheckedAccount` (Core asset)
- `auction: Account<Auction>` (PDA, `close = seller`)
- `system_program`
- `core_program` (MPL Core) [file:1]

**What it does:**

- Checks:
  - `seller` equals `auction.seller`.
  - `auction.active == 1`.
  - `auction.highest_bid == 0` (no one has bid yet). [file:1]
- Transfers the Core NFT back to the seller:
  - Calls `TransferV1CpiBuilder`:
    - `authority = auction PDA`.
    - `new_owner = seller`.
- Logs a message like:
  - “Auction for asset `<key>` cancelled by seller `<key>` (no bids).”
- Sets `auction.active = 0`.
- Because of `close = seller`, Anchor then:
  - Closes the `auction` PDA.
  - Sends any remaining rent lamports back to the seller. [file:1]

This keeps the cancel path simple, predictable, and safe before you add a more advanced “cancel with refund” using a dedicated SOL vault PDA.

---

## Testing & local/devnet notes

- **Local / Anchor tests**
  - Use `@metaplex-foundation/mpl-core` and Umi to mint real Core NFTs during tests.
  - Use a Phantom wallet JSON file as the payer and Core asset owner.
  - Derive the `auction` PDA with `[b"auction", assetPubkey.toBuffer()]` on the client, matching seeds on-chain.
  - Run full flows:
    - Mint Core NFT → list → bid → settle.
    - Mint Core NFT → list → cancel (no bids). [file:1]

- **Switchboard**
  - The Switchboard On-Demand VRF integration is kept separate so the marketplace logic can still be tested independently on devnet, even if the randomness queue is only wired for localnet or a particular devnet queue. [file:1]

---

## Future improvements

Some planned or obvious next steps:

- **Cancel after bids with refunds**
  - Introduce a dedicated SOL vault PDA (system-owned, no data) per auction.
  - Move all bid SOL into that vault and handle refunds + seller payouts via SystemProgram transfers from the vault.

- **Multiple packs**
  - Use a `pack_id` or admin + nonce in seeds to support arbitrarily many packs with different configurations. [file:1]

- **Frontend**
  - Next.js / React UI that:
    - Shows available packs.
    - Lets users buy and open packs.
    - Renders inventory by reading all `UserPack` accounts for the connected wallet.
    - Lists active auctions and allows bidding/settling/cancelling. [file:1]

- **Full Core custody for pack rewards**
  - Replace “fake” IDs in `UserPack.nft_mints` with real Core NFTs drawn from a pool, then pipe those directly into the auction listing flow.

---

## Summary

**Gacha Gotcha** ties together:

- Pack-based randomness (with Switchboard).
- Persistent per-user state for opened packs and rewards.
- A working on-chain marketplace using **Metaplex Core NFTs** and an **auction PDA as a SOL vault**, all tested end-to-end on devnet.

It’s a compact but realistic example of a game + marketplace architecture on Solana, written in Anchor and designed to be easy to extend. [file:1]
