use crate::{
    engine::pay::process_express_checkout, engine::register::process_register_merchant,
    engine::withdraw::process_withdraw_payment, instruction::PaymentProcessorInstruction,
    engine::subscribe::process_subscribe
};
use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
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
            PaymentProcessorInstruction::RegisterMerchant { seed, fee, data } => {
                msg!("Instruction: RegisterMerchant");
                process_register_merchant(program_id, accounts, seed, fee, data)
            }
            PaymentProcessorInstruction::ExpressCheckout {
                amount,
                order_id,
                secret,
                data,
            } => {
                msg!("Instruction: ExpressCheckout");
                process_express_checkout(program_id, accounts, amount, order_id, secret, data)
            }
            PaymentProcessorInstruction::Withdraw => {
                msg!("Instruction: Withdraw");
                process_withdraw_payment(program_id, accounts)
            }
            PaymentProcessorInstruction::Subscribe { name, data } => {
                msg!("Instruction: Subscribe");
                process_subscribe(program_id, accounts, name, data)
            }
        }
    }
}
