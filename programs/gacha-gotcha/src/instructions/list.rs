use anchor_lang::prelude::*;
use crate::state::Auction;
use mpl_core::{ID as CORE_PROGRAM_ID, instructions::TransferV1CpiBuilder};

#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mut,
    )]
    /// CHECK: type & ownership checked by Core CPI later
    pub asset: UncheckedAccount<'info>,

    #[account(
        init,
        payer = seller,
        seeds = [b"auction", asset.key().as_ref()],
        space = Auction::DISCRIMINATOR.len() + Auction::INIT_SPACE, 
        bump
    )]
    pub auction: Account<'info, Auction>,

    pub system_program: Program<'info, System>, 
    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: checked by core
    pub core_program: UncheckedAccount<'info>,
}

impl<'info> List<'info> {
    pub fn list(&mut self, min_bid: u64, duration: i64, bump: u8) -> Result<()> {
        let clock = Clock::get()?.unix_timestamp;

        self.auction.set_inner(Auction {
            seller: self.seller.key(),
            nft: self.asset.key(),
            highest_bidder: Pubkey::default(),
            highest_bid: 0,
            minimum_bid: min_bid,
            end_time: clock + duration,
            bump ,
            active: 1, 
        });

        TransferV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .authority(Some(&self.seller.to_account_info()))
            .new_owner(&self.auction.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .payer(&self.seller.to_account_info())
            .invoke()?;

        Ok(())
    }
}
