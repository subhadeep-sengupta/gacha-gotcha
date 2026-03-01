use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as CORE_PROGRAM_ID};

use crate::{error::AuctionError, state::Auction};

#[derive(Accounts)]
pub struct Settle<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    /// CHECK
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    #[account(mut)]
    pub winner: SystemAccount<'info>,

    #[account(
        mut,
        close = seller,
        seeds = [b"auction", asset.key().as_ref()],
        bump = auction.bump
    )]
    pub auction: Account<'info, Auction>,

    pub system_program: Program<'info, System>,

    ///CHECK
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl<'info> Settle<'info> {
    pub fn settle(&mut self) -> Result<()> {
        let clock = Clock::get()?.unix_timestamp;

        require!(self.auction.active == 1, AuctionError::Ended);
        require!(clock >= self.auction.end_time, AuctionError::Ended);
        require!(self.auction.highest_bid > 0, AuctionError::BidTooLow);

        // Ensure correct accounts were passed
        require_keys_eq!(
            self.seller.key(),
            self.auction.seller,
            AuctionError::Unauthorized
        );
        require_keys_eq!(
            self.winner.key(),
            self.auction.highest_bidder,
            AuctionError::Unauthorized
        );

        let seeds: &[&[u8]] = &[
            b"auction",
            &self.asset.key().to_bytes(),
            &[self.auction.bump],
        ];

        let signer_seeds = &[seeds];

        let amount = self.auction.highest_bid;

        **self.auction.to_account_info().try_borrow_mut_lamports()? -= amount;

        **self.seller.to_account_info().try_borrow_mut_lamports()? += amount;

        TransferV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .payer(&self.seller.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .authority(Some(&self.auction.to_account_info()))
            .new_owner(&self.winner.to_account_info())
            .invoke_signed(signer_seeds)?;

        Ok(())
    }
}
