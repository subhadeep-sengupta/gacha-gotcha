use anchor_lang::prelude::*;
use switchboard_on_demand::accounts::RandomnessAccountData;

use crate::error::GachaError;
use crate::state::PackConfig;
use crate::state::PackRarityConfig;
use crate::state::PackStatus;
use crate::state::UserPack;

#[derive(Accounts)]
pub struct OpenPack<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"userpack", pack.key().as_ref(), buyer.key().as_ref()],
        bump = user_pack.bump
    )]
    pub user_pack: Account<'info, UserPack>,
    #[account(
        seeds = [b"pack"],
        bump = pack.bump
    )]
    pub pack: Account<'info, PackConfig>,
    #[account(
        mut,
        constraint = rarity.pack == pack.key() @ GachaError::InvalidRarity
    )]
    pub rarity: Account<'info, PackRarityConfig>,
    /// CHECK: Switchboard randomness account
    pub randomness_account: AccountInfo<'info>,
}

impl<'info> OpenPack<'info> {
    pub fn open_pack(&mut self, program_id: &Pubkey) -> Result<()> {
        let clock = Clock::get()?;
        let user_pack = &mut self.user_pack;
        let pack = &mut self.pack;
        let rarity = &mut self.rarity;

        require!(
            user_pack.status == PackStatus::Committed,
            GachaError::NotCommitted
        );
        require_keys_eq!(
            self.randomness_account.key(),
            user_pack.randomness_account,
            GachaError::InvalidRandomness
        );

        let randomness_data = RandomnessAccountData::parse(self.randomness_account.data.borrow())
            .map_err(|_| GachaError::InvalidRandomness)?;

        require_eq!(
            randomness_data.seed_slot,
            user_pack.commit_slot,
            GachaError::RandomnessExpired
        );

        let random_bytes = randomness_data
            .get_value(clock.slot)
            .map_err(|_| GachaError::RandomnessNotReady)?;
        // NOTE: This will only succeed when `clock.slot == reveal_slot` for this randomness [web:9].

        for i in 0..pack.pack_size as usize {
            let byte = random_bytes[i];
            let rarity_idx = draw_rarity(byte, &rarity.weights)?;
            user_pack.revealed[i] = rarity_idx as u8;

            let (nft_mint, _bump) = Pubkey::find_program_address(
                &[
                    b"nft",
                    &i.to_le_bytes(),
                    &rarity_idx.to_le_bytes(),
                    &byte.to_le_bytes(),
                ],
                program_id,
            );
            user_pack.nft_mints[i] = nft_mint;
            msg!("Card {}: rarity {}, mint {}", i, rarity_idx, nft_mint);
        }

        user_pack.status = PackStatus::Opened;
        msg!("Pack opened! Rarities: {:?}", user_pack.revealed);

        Ok(())
    }
}

fn draw_rarity(byte: u8, weights: &[u16; 4]) -> Result<usize> {
    let total: u16 = weights.iter().sum();
    let roll = (byte as u16) % total;
    let mut acc = 0u16;
    for (i, w) in weights.iter().enumerate() {
        acc += *w;
        if roll < acc {
            return Ok(i);
        }
    }
    Ok(3) // fallback to legendary
}
