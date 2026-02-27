use anchor_lang::prelude::*;

use crate::{PackConfig, PackRarityConfig, PackSaleStatus};

#[derive(Accounts)]
pub struct CreatePack<'info> {
    #[account(
        init,
        payer = admin,
        seeds = [b"pack"],
        bump,
        space = 8 + 32 + 8 + 1 + 4 + 4 + 1 + 1 + 37
    )]
    pub pack: Account<'info, PackConfig>,
    #[account(
        init,
        payer = admin,
        seeds = [b"rarity", pack.key().as_ref()],
        bump,
        space = 96
    )]
    pub rarity: Account<'info, PackRarityConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreatePack<'info> {
    pub fn create_pack(&mut self) -> Result<()> {
        self.pack.set_inner(PackConfig {
            admin: self.admin.key(),
            price_lamports: 100_000_000,
            pack_size: 5,
            total_supply: 100,
            sold_count: 0,
            sale_status: PackSaleStatus::Live,
            bump: self.pack.bump,
        });

        self.rarity.set_inner(PackRarityConfig {
            pack: self.pack.key(),
            weights: [70u16, 20u16, 9u16, 1u16],
            remaining_per_rarity: [70u32, 20u32, 9u32, 1u32],
            bump: self.rarity.bump,
        });

        msg!("Pack created with key: {}", self.pack.key());
        Ok(())
    }
}
