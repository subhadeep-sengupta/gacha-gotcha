use anchor_lang::prelude::*;

#[account]
pub struct PackConfig {
    pub admin: Pubkey,
    pub price_lamports: u64,
    pub pack_size: u8,
    pub total_supply: u32,
    pub sold_count: u32,
    pub sale_status: PackSaleStatus,
    pub bump: u8,
}

#[account]
pub struct PackRarityConfig {
    pub pack: Pubkey,
    pub weights: [u16; 4],
    pub remaining_per_rarity: [u32; 4],
    pub bump: u8,
}

#[account]
pub struct UserPack {
    pub owner: Pubkey,
    pub pack: Pubkey,
    pub status: PackStatus,
    pub randomness_account: Pubkey,
    pub commit_slot: u64,
    pub revealed: [u8; 5],
    pub bump: u8,
    pub nft_mints: [Pubkey; 5],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PackSaleStatus {
    NotStarted,
    Live,
    Paused,
    Ended,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PackStatus {
    Unopened,
    Committed,
    Opened,
}

#[account]
#[derive(InitSpace)]
pub struct Auction {
    pub seller: Pubkey,
    pub nft: Pubkey,
    pub highest_bidder: Pubkey,
    pub highest_bid: u64,
    pub minimum_bid: u64,
    pub end_time: i64,
    pub bump: u8,
    pub active: u8,
}
