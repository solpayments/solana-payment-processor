use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

use crate::error::PaymentProcessorError::InvalidInstruction;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
pub enum PaymentProcessorInstruction {
    /// Register for a merchant account.
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person initializing the loan
    /// 1. `[]` The payments receipt
    /// 2. `[writable]` The merchant account.  Owned by this program
    /// 3. `[]` The rent sysvar
    RegisterMerchant,
}

impl PaymentProcessorInstruction {
    /// Unpacks a byte buffer into a [PaymentProcessorInstruction](enum.PaymentProcessorInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, _rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => Self::RegisterMerchant,
            _ => return Err(InvalidInstruction.into()),
        })
    }

    pub fn pack_into_vec(&self) -> Vec<u8> {
        self.try_to_vec().expect("try_to_vec")
    }
}

/// Creates an 'RegisterMerchant' instruction.
pub fn register_merchant(
    program_id: Pubkey,
    signer_pubkey: Pubkey,
    payments_acc_pubkey: Pubkey,
    merchant_acc_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new_readonly(payments_acc_pubkey, false),
            AccountMeta::new(merchant_acc_pubkey, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: PaymentProcessorInstruction::RegisterMerchant
        .pack_into_vec(),
    }
}