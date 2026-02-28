use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod state;

use instructions::*;
use state::*;

declare_id!("6AkHVw9JWm2Ge1S47KmeETiFcnFXBMLr5UMKPQGQm3Wf");

#[program]
pub mod gacha_gotcha {
    use super::*;

    pub fn create_pack(ctx: Context<CreatePack>) -> Result<()> {
        ctx.accounts.create_pack()
    }

    pub fn buy_pack(ctx: Context<BuyPack>) -> Result<()> {
        ctx.accounts.buy_pack(ctx.bumps.user_pack)
    }

    pub fn commit_open(ctx: Context<CommitOpen>, randomness_account: Pubkey) -> Result<()> {
        ctx.accounts.commit_open(randomness_account)
    }

    pub fn open_pack(ctx: Context<OpenPack>) -> Result<()> {
        ctx.accounts.open_pack(ctx.program_id)
    }

    pub fn list(ctx: Context<List>, min_bid: u64, duration: i64) -> Result<()> {
        ctx.accounts.list(min_bid, duration, ctx.bumps.auction)
    }
}
