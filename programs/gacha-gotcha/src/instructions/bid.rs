use crate::{error::AuctionError, state::Auction};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use mpl_core::ID as CORE_PROGRAM_ID;

#[derive(Accounts)]
pub struct Bid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,

    /// CHECK: the asset is given from the client side
    pub asset: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: will be checked against auction.highest_bidder in handler
    #[account(mut)]
    pub previous_highest_bidder: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"auction", asset.key().as_ref()],
        bump = auction.bump
    )]
    pub auction: Account<'info, Auction>,

    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: checked by core
    pub core_program: UncheckedAccount<'info>,
}

impl<'info> Bid<'info> {
    pub fn bid(&mut self, bid: u64) -> Result<()> {
        require!(self.auction.active == 1, AuctionError::AuctionNotActive);
        require!(bid >= self.auction.minimum_bid, AuctionError::BidTooLow);
        require!(bid > self.auction.highest_bid, AuctionError::BidTooLow);

        let bidder_to_auction = Transfer {
            from: self.bidder.to_account_info(),
            to: self.auction.to_account_info(),
        };
        let cpi_ctx_ba = CpiContext::new(self.system_program.to_account_info(), bidder_to_auction);
        transfer(cpi_ctx_ba, bid)?;

        // 3. If there was a previous highest bid, refund it via PDA-signed CPI
        if self.auction.highest_bid > 0 {
            // Security: ensure this is really the old highest bidder
            require_keys_eq!(
                self.previous_highest_bidder.key(),
                self.auction.highest_bidder,
                AuctionError::Unauthorized
            );

            // Use as_ref(), not to_bytes(), so we don't take a ref to a temporary
            let signer_seeds: &[&[&[u8]]] = &[&[
                b"auction",
                &self.asset.key().to_bytes(),
                &[self.auction.bump],
            ]];

            let auction_to_prev = Transfer {
                from: self.auction.to_account_info(),
                to: self.previous_highest_bidder.to_account_info(),
            };

            let cpi_ctx_apb = CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                auction_to_prev,
                signer_seeds,
            );

            transfer(cpi_ctx_apb, self.auction.highest_bid)?;
        }

        // 4. Update state for both branches
        self.auction.highest_bid = bid;
        self.auction.highest_bidder = self.bidder.key();

        Ok(())
    }
}
