use crate::{
    error::PaymentProcessorError,
    instruction::PaymentProcessorInstruction,
    state::{MerchantAccount, Serdes},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized},
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token;

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = PaymentProcessorInstruction::unpack(instruction_data)?;

    match instruction {
        PaymentProcessorInstruction::RegisterMerchant => {
            msg!("Instruction: RegisterMerchant");
            process_register_merchant(program_id, accounts)
        }
        PaymentProcessorInstruction::ExpressCheckout {
            amount,
            merchant_token_pubkey,
            order_id,
        } => {
            msg!("Instruction: ExpressCheckout");
            process_express_checkout(
                program_id,
                accounts,
                amount,
                merchant_token_pubkey,
                order_id,
            )
        }
    }
}

pub fn process_register_merchant(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // ensure merchant account is owned by this program
    if *merchant_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    // ensure merchant account is rent exempt
    if !rent.is_exempt(merchant_info.lamports(), MerchantAccount::LEN) {
        return Err(PaymentProcessorError::NotRentExempt.into());
    }

    // get the merchant account
    let mut merchant_account = MerchantAccount::unpack(&merchant_info.data.borrow())?;
    if merchant_account.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // create the MerchantAccount object
    msg!("Saving merchant account information...");
    merchant_account.is_initialized = true;
    merchant_account.merchant_pubkey = merchant_info.key.to_bytes();
    MerchantAccount::pack(&merchant_account, &mut merchant_info.data.borrow_mut());

    Ok(())
}

pub fn process_express_checkout(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    merchant_token_pubkey: [u8; 32],
    order_id: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let payer_token_info = next_account_info(account_info_iter)?;
    let merchant_acc_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // get the merchant account
    let mut merchant_account = MerchantAccount::unpack(&merchant_acc_info.data.borrow())?;
    if merchant_account.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    // ensure payer token account is owned by this program
    if *payer_token_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    // ensure merchant account is owned by this program
    if *merchant_acc_info.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}
