use crate::{
    engine::register::process_register_merchant,
    engine::withdraw::process_withdraw_payment,
    engine::pay::process_express_checkout,
    instruction::PaymentProcessorInstruction,
};
use borsh::BorshDeserialize;
use solana_program::{
    account_info::{AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Processes the instruction
impl PaymentProcessorInstruction {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = PaymentProcessorInstruction::try_from_slice(&instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        match instruction {
            PaymentProcessorInstruction::RegisterMerchant { seed } => {
                msg!("Instruction: RegisterMerchant");
                process_register_merchant(program_id, accounts, seed)
            }
            PaymentProcessorInstruction::ExpressCheckout {
                amount,
                order_id,
                secret,
            } => {
                msg!("Instruction: ExpressCheckout");
                process_express_checkout(program_id, accounts, amount, order_id, secret)
            }
            PaymentProcessorInstruction::Withdraw => {
                msg!("Instruction: Withdraw");
                process_withdraw_payment(program_id, accounts)
            } // _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
