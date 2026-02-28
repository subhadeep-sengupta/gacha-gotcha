use anchor_lang::prelude::*;

#[error_code]
pub enum GachaError {
    #[msg("Sale not live")]
    SaleNotLive,
    #[msg("Pack sold out")]
    SoldOut,
    #[msg("Already committed")]
    AlreadyCommitted,
    #[msg("Randomness expired")]
    RandomnessExpired,
    #[msg("Randomness already revealed")]
    RandomnessRevealed,
    #[msg("Not committed")]
    NotCommitted,
    #[msg("Invalid randomness")]
    InvalidRandomness,
    #[msg("Invalid rarity")]
    InvalidRarity,
    #[msg("Randomness not ready")]
    RandomnessNotReady,
}

#[error_code]
pub enum AuctionError {
    #[msg("Auction already active")]
    AlreadyActive,
    #[msg("Auction ended")]
    Ended,
    #[msg("Bid too low")]
    BidTooLow,
    #[msg("Unauthorized")]
    Unauthorized,
}
