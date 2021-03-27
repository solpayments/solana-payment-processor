use crate::{
    error::PaymentProcessorError,
    instruction::PaymentProcessorInstruction,
    state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
};
use borsh::BorshDeserialize;
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token;
use spl_token::{
    instruction::initialize_account,
    state::{Account as TokenAccount, AccountState, Mint},
};
use std::convert::TryInto;

pub const PAYMENT_PROCESSOR: &str = "payment-processor";

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
            PaymentProcessorInstruction::RegisterMerchant => {
                msg!("Instruction: RegisterMerchant");
                process_register_merchant(program_id, accounts)
            }
            PaymentProcessorInstruction::ExpressCheckout { amount, order_id } => {
                msg!("Instruction: ExpressCheckout");
                process_express_checkout(program_id, accounts, amount, order_id)
            }
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

pub fn process_register_merchant(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let system_sysvar_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // test that merchant account pubkey is correct
    let address_with_seed =
        Pubkey::create_with_seed(signer_info.key, PAYMENT_PROCESSOR, program_id)?;
    if *merchant_info.key != address_with_seed {
        return Err(ProgramError::IncorrectProgramId);
    }
    // try create merchant account
    let create_account_ix = system_instruction::create_account_with_seed(
        signer_info.key,
        merchant_info.key,
        signer_info.key,
        PAYMENT_PROCESSOR,
        Rent::default().minimum_balance(MerchantAccount::LEN),
        MerchantAccount::LEN.try_into().unwrap(),
        program_id,
    );
    msg!("Creating merchant account on chain...");
    invoke(
        &create_account_ix,
        &[
            signer_info.clone(),
            merchant_info.clone(),
            signer_info.clone(),
            system_sysvar_info.clone(),
        ],
    )?;

    // ensure merchant account is rent exempt
    if !rent.is_exempt(merchant_info.lamports(), MerchantAccount::LEN) {
        return Err(PaymentProcessorError::NotRentExempt.into());
    }

    // get the merchant account data
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
    order_id: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let payer_token_info = next_account_info(account_info_iter)?;
    let order_acc_info = next_account_info(account_info_iter)?;
    let merchant_acc_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_sysvar_info)?;
    let timestamp = &Clock::from_account_info(clock_sysvar_info)?.unix_timestamp;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // get the merchant account
    let merchant_account = MerchantAccount::unpack(&merchant_acc_info.data.borrow())?;
    if !merchant_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // ensure payer token account is owned by token program
    if *payer_token_info.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    // ensure merchant account is owned by this program
    if *merchant_acc_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    // ensure order account is owned by this program
    if *order_acc_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    let (pda, nonce) = Pubkey::find_program_address(&[b"payment-processor"], program_id);
    // get the order account
    let mut order_account = order_acc_info.try_borrow_mut_data()?;
    msg!("Saving order information...");
    let order = OrderAccount {
        status: OrderStatus::Paid as u8,
        created: *timestamp,
        modified: *timestamp,
        merchant_pubkey: merchant_acc_info.key.to_bytes(),
        mint_pubkey: merchant_acc_info.key.to_bytes(),
        payer_pubkey: signer_info.key.to_bytes(),
        expected_amount: amount,
        paid_amount: 0,
        fee_amount: 0,
        order_id,
    };

    order.pack(&mut order_account);

    Ok(())
}
