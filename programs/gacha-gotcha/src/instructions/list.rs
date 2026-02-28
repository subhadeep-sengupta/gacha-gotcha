use anchor_lang::prelude::*;
use anchor_spl::token::{ Mint, Token, TokenAccount, Transfer, transfer};

use crate::state::Auction;

#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,

    pub nft_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = seller,
        seeds = [b"auction", nft_mint.key().as_ref()],
        space = Auction::DISCRIMINATOR.len() + Auction::INIT_SPACE, 
        bump
    )]
    pub auction: Account<'info, Auction>,

    #[account(
        init,
        payer = seller,
        token::mint = nft_mint,
        token::authority = auction,
    )]
    pub nft_escrow: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = seller_token_account.owner == seller.key(),
        constraint = seller_token_account.mint == nft_mint.key(),
    )]
    pub seller_token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

impl<'info> List<'info> {
    pub fn list(&mut self, min_bid: u64, duration: i64, bump: u8) -> Result<()> {
        let clock = Clock::get()?.unix_timestamp;

        self.auction.set_inner(Auction {
            seller: self.seller.key(),
            nft: self.nft_mint.key(),
            highest_bidder: Pubkey::default(),
            highest_bid: 0,
            minimum_bid: min_bid,
            end_time: clock + duration,
            bump ,
            active: 1, 
        });

        let accounts = Transfer{
            from: self.seller_token_account.to_account_info(),
            to: self.nft_escrow.to_account_info(),
            authority: self.seller.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), accounts);

        transfer(cpi_ctx, 1)?;

        Ok(())
    }
}
