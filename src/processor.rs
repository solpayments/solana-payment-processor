use crate::{
    error::PaymentProcessorError,
    instruction::PaymentProcessorInstruction,
    state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
    utils::get_amounts,
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
    sysvar::{rent::Rent, Sysvar},
};
use spl_token;
use spl_token::state::{Account as TokenAccount, AccountState, Mint};
use spl_associated_token_account;

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
    order_id: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let payer_token_info = next_account_info(account_info_iter)?;
    let order_acc_info = next_account_info(account_info_iter)?;
    let order_token_acc_info = next_account_info(account_info_iter)?;
    let merchant_acc_info = next_account_info(account_info_iter)?;
    let mint_acc_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
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
    // get pda and nonce
    // let (pda, nonce) = Pubkey::find_program_address(&[b"loan"], program_id);
    // Get mint details and verify that they match token account
    let payer_token_account_data = TokenAccount::unpack(&payer_token_info.data.borrow())?;
    if mint_acc_info.key != &payer_token_account_data.mint {
        return Err(ProgramError::IncorrectProgramId);
    }
    // let x = spl_token::instruction::initialize_account(
    //     token_program_info.key,
    //     &pda,
    //     &payer_token_account_data.mint,
    //     program_id
    // );
    // let xx = spl_associated_token_account::get_associated_token_address(
    //     order_acc_info.key,
    //     &payer_token_account_data.mint
    // );
    let create_order_token_acc_ix = spl_associated_token_account::create_associated_token_account(
        signer_info.key,
        order_acc_info.key,
        &payer_token_account_data.mint
    );
    invoke(
        &create_order_token_acc_ix,
        &[
            signer_info.clone(),
            order_token_acc_info.clone(),
            order_acc_info.clone(),
            mint_acc_info.clone(),
            system_program_info.clone(),
            token_program_info.clone(),
            rent_sysvar_info.clone(),
        ],
    )?;
    // Get fee and take home amount
    let (take_home_amount, fee_amount) = get_amounts(amount);

    // get the order account
    // TODO: ensure this account is not already initialized
    let mut order_account_data = order_acc_info.try_borrow_mut_data()?;
    msg!("Saving order information...");
    let order = OrderAccount {
        status: OrderStatus::Paid as u8,
        created: *timestamp,
        modified: *timestamp,
        merchant_pubkey: merchant_acc_info.key.to_bytes(),
        mint_pubkey: payer_token_account_data.mint.to_bytes(),
        payer_pubkey: signer_info.key.to_bytes(),
        expected_amount: amount,
        paid_amount: amount,
        take_home_amount,
        fee_amount: fee_amount as u64,
        order_id,
    };

    order.pack(&mut order_account_data);

    Ok(())
}
