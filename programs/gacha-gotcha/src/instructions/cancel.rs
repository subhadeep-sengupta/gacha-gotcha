use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as CORE_PROGRAM_ID};

use crate::{error::AuctionError, state::Auction};

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    /// CHECK
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    #[account(
        mut,
        close = seller,
        seeds = [b"auction", asset.key().as_ref()],
        bump = auction.bump,
    )]
    pub auction: Account<'info, Auction>,

    pub system_program: Program<'info, System>,

    ///CHECK
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl<'info> Cancel<'info> {
    pub fn cancel(&mut self) -> Result<()> {
        require_keys_eq!(
            self.seller.key(),
            self.auction.seller,
            AuctionError::Unauthorized
        );

        require!(self.auction.active == 1, AuctionError::Ended);

        require!(self.auction.highest_bid == 0, AuctionError::BidTooLow);

        let seeds: &[&[u8]] = &[
            b"auction",
            &self.asset.key().to_bytes(),
            &[self.auction.bump],
        ];
        let signer_seeds = &[seeds];

        TransferV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .authority(Some(&self.auction.to_account_info()))
            .new_owner(&self.seller.to_account_info())
            .payer(&self.seller.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signer_seeds)?;

        msg!(
            "Auction for asset {} cancelled by seller {}",
            self.asset.key(),
            self.seller.key()
        );
        self.auction.active = 0;
        Ok(())
    }
}
