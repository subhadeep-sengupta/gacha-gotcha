use anchor_lang::prelude::*;
use switchboard_on_demand::accounts::RandomnessAccountData;

use crate::error::GachaError;
use crate::state::PackConfig;
use crate::state::PackStatus;
use crate::state::UserPack;

#[derive(Accounts)]
pub struct CommitOpen<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        seeds = [b"pack"],
        bump = pack.bump,
    )]
    pub pack: Account<'info, PackConfig>,

    #[account(
        mut,
        seeds = [b"userpack", pack.key().as_ref(), buyer.key().as_ref()],
        bump = user_pack.bump,
    )]
    pub user_pack: Account<'info, UserPack>,

    /// CHECK: Switchboard randomness account
    pub randomness_account: AccountInfo<'info>,
}

impl<'info> CommitOpen<'info> {
    pub fn commit_open(&mut self, randomness_account: Pubkey) -> Result<()> {
        let clock = Clock::get()?;
        let user_pack = &mut self.user_pack;

        require!(
            user_pack.status == PackStatus::Unopened,
            GachaError::AlreadyCommitted
        );

        // Ensure the account passed in matches the pubkey arg
        require_keys_eq!(
            self.randomness_account.key(),
            randomness_account,
            GachaError::InvalidRandomness
        );

        let randomness_data = RandomnessAccountData::parse(self.randomness_account.data.borrow())
            .map_err(|_| GachaError::InvalidRandomness)?;

        // Verify fresh commit – this matches the coinflip tutorial pattern [web:8]
        require_eq!(
            randomness_data.seed_slot,
            clock.slot - 1,
            GachaError::RandomnessExpired
        );
        require!(
            randomness_data.get_value(clock.slot).is_err(),
            GachaError::RandomnessRevealed
        );

        user_pack.randomness_account = self.randomness_account.key();
        user_pack.commit_slot = randomness_data.seed_slot;
        user_pack.status = PackStatus::Committed;

        msg!(
            "Pack committed for randomness: {}",
            user_pack.randomness_account
        );
        Ok(())
    }
}
