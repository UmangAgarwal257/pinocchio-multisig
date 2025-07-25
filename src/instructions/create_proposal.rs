use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey,
    sysvars::{Sysvar, clock::Clock, rent::Rent},
};

use crate::state::proposal::{ProposalState, ProposalStatus};
pub fn process_create_proposal_instruction(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [creator, proposal_account, multisig_account, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let bump = unsafe { *(data.as_ptr() as *const u8) };
    let bump_bytes = bump.to_le_bytes();

    let seed = [
        (b"proposal"),
        multisig_account.key().as_slice(),
        bump_bytes.as_ref(),
    ];

    let seeds = &seed[..];
    let pda = pubkey::checked_create_program_address(seeds, &crate::ID)
        .map_err(|_| ProgramError::InvalidSeeds)?;

    if &pda != proposal_account.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    if proposal_account.owner() != &crate::ID {
        pinocchio_system::instructions::CreateAccount {
            from: creator,
            to: proposal_account,
            lamports: Rent::get()?.minimum_balance(ProposalState::LEN),
            space: ProposalState::LEN as u64,
            owner: &crate::ID,
        }
        .invoke()?;
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let proposal = ProposalState::from_account_info(proposal_account)?;

    // Check if expity is in the future btw
    let expiry = unsafe { *(data.as_ptr().add(16) as *const u64) };
    let current_time = Clock::get()?.unix_timestamp as u64;
    if expiry <= current_time {
        return Err(ProgramError::InvalidInstructionData);
    }

    proposal.bump = bump;
    proposal.proposal_id = Clock::get()?.slot;
    proposal.result = ProposalStatus::Draft;
    proposal.created_time = Clock::get()?.slot;
    proposal.expiry = expiry;

    proposal.active_members = [pubkey::Pubkey::default(); 10];
    proposal.votes = [0u8; 10]; //10 would be max number of active members

    Ok(())
}
