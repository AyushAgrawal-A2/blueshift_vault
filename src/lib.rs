#![no_std]

use pinocchio::{
    cpi::{Seed, Signer},
    entrypoint,
    error::ProgramError,
    nostd_panic_handler,
    sysvars::{rent::Rent, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::instructions::Transfer;

entrypoint!(process_instruction);
nostd_panic_handler!();

pub const ID: Address = Address::from_str_const("22222222222222222222222222222222222222222222");

fn process_instruction(
    _program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        Some((&Deposit::DESCRIMINATOR, data)) => Deposit::try_from((data, accounts))?.process(),
        Some((&Withdraw::DISCRIMINATOR, _)) => Withdraw::try_from(accounts)?.process(),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

struct DepositAccounts<'a> {
    owner: &'a AccountView,
    vault: &'a AccountView,
}
impl<'a> TryFrom<&'a [AccountView]> for DepositAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [owner, vault, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        if !owner.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !vault.owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if vault.lamports().ne(&0) {
            return Err(ProgramError::InvalidAccountData);
        }
        let (vault_key, _) =
            Address::derive_program_address(&[b"vault", owner.address().as_ref()], &crate::ID)
                .ok_or(ProgramError::InvalidSeeds)?;
        if vault.address().ne(&vault_key) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(Self { owner, vault })
    }
}

struct DepositInstructionData {
    amount: u64,
}
impl<'a> TryFrom<&'a [u8]> for DepositInstructionData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != size_of::<u64>() {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = u64::from_le_bytes(data.try_into().unwrap());
        if amount <= Rent::get()?.try_minimum_balance(0)? {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(Self { amount })
    }
}

struct Deposit<'a> {
    accounts: DepositAccounts<'a>,
    instruction_data: DepositInstructionData,
}
impl<'a> TryFrom<(&'a [u8], &'a mut [AccountView])> for Deposit<'a> {
    type Error = ProgramError;
    fn try_from((data, accounts): (&'a [u8], &'a mut [AccountView])) -> Result<Self, Self::Error> {
        let accounts = DepositAccounts::try_from(&*accounts)?;
        let instruction_data = DepositInstructionData::try_from(data)?;
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}
impl<'a> Deposit<'a> {
    const DESCRIMINATOR: u8 = 0;
    fn process(&mut self) -> ProgramResult {
        Transfer {
            from: self.accounts.owner,
            to: self.accounts.vault,
            lamports: self.instruction_data.amount,
        }
        .invoke()?;
        Ok(())
    }
}

struct WithdrawAccounts<'a> {
    owner: &'a AccountView,
    vault: &'a AccountView,
    bumps: [u8; 1],
}
impl<'a> TryFrom<&'a [AccountView]> for WithdrawAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [owner, vault, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        if !owner.is_signer() {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if !vault.owned_by(&pinocchio_system::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if vault.lamports().eq(&0) {
            return Err(ProgramError::InsufficientFunds);
        }
        let (vault_key, bump) =
            Address::derive_program_address(&[b"vault", owner.address().as_ref()], &crate::ID)
                .ok_or(ProgramError::InvalidSeeds)?;
        if vault.address().ne(&vault_key) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(Self {
            owner,
            vault,
            bumps: [bump],
        })
    }
}

struct Withdraw<'a> {
    accounts: WithdrawAccounts<'a>,
}
impl<'a> TryFrom<&'a mut [AccountView]> for Withdraw<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a mut [AccountView]) -> Result<Self, Self::Error> {
        let accounts = WithdrawAccounts::try_from(&*accounts)?;
        Ok(Self { accounts })
    }
}
impl<'a> Withdraw<'a> {
    const DISCRIMINATOR: u8 = 1;
    fn process(&mut self) -> ProgramResult {
        let seeds = [
            Seed::from(b"vault"),
            Seed::from(self.accounts.owner.address().as_ref()),
            Seed::from(&self.accounts.bumps),
        ];
        let signers = [Signer::from(&seeds)];
        Transfer {
            from: self.accounts.vault,
            to: self.accounts.owner,
            lamports: self.accounts.vault.lamports(),
        }
        .invoke_signed(&signers)?;
        Ok(())
    }
}
