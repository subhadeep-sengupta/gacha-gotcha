use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::error::GachaError;
use crate::state::PackConfig;
use crate::state::PackSaleStatus;
use crate::state::PackStatus;
use crate::state::UserPack;

#[derive(Accounts)]
pub struct BuyPack<'info> {
    #[account(
        mut,
        seeds = [b"pack"],
        bump = pack.bump
    )]
    pub pack: Account<'info, PackConfig>,
    #[account(
        init,
        payer = buyer,
        seeds = [b"userpack", pack.key().as_ref(), buyer.key().as_ref()],
        space = 8 + 32 + 32 + 1 + 32 + 8 + 5 + (32 * 5) + 1,
        bump
    )]
    pub user_pack: Account<'info, UserPack>,
    /// CHECK: treasury
    #[account(mut)]
    pub treasury: UncheckedAccount<'info>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> BuyPack<'info> {
    pub fn buy_pack(&mut self, bump: u8) -> Result<()> {
        let pack = &mut self.pack;
        require!(
            pack.sale_status == PackSaleStatus::Live,
            GachaError::SaleNotLive
        );
        require!(pack.sold_count < pack.total_supply, GachaError::SoldOut);

        // Transfer SOL payment
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.buyer.to_account_info(),
                    to: self.treasury.to_account_info(),
                },
            ),
            pack.price_lamports,
        )?;

        // Create user pack
        self.user_pack.set_inner(UserPack {
            owner: self.buyer.key(),
            pack: pack.key(),
            status: PackStatus::Unopened,
            randomness_account: Pubkey::default(),
            commit_slot: 0,
            revealed: [0u8; 5],
            bump,
            nft_mints: [Pubkey::default(); 5],
        });

        pack.sold_count += 1;
        msg!("User bought pack #{}", pack.sold_count);
        Ok(())
    }
}
