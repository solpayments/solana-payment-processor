use crate::{
    error::PaymentProcessorError,
    instruction::PaymentProcessorInstruction,
    state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
    utils::{get_amounts, get_order_account_size},
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
// use spl_associated_token_account;
use spl_token::{
    self,
    instruction::initialize_account,
    state::{Account as TokenAccount, AccountState, Mint},
};
use std::convert::TryInto;

pub const MERCHANT: &str = "merchant";
/// maximum length of derived `Pubkey` seed
pub const MAX_SEED_LEN: usize = 32;

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
    let address_with_seed = Pubkey::create_with_seed(signer_info.key, MERCHANT, program_id)?;
    if *merchant_info.key != address_with_seed {
        return Err(ProgramError::InvalidSeeds);
    }
    // try create merchant account
    let create_account_ix = system_instruction::create_account_with_seed(
        signer_info.key,
        merchant_info.key,
        signer_info.key,
        MERCHANT,
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
        return Err(ProgramError::AccountNotRentExempt);
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
    let order_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let seller_token_info = next_account_info(account_info_iter)?;
    let buyer_token_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;

    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // get the merchant account
    let merchant_account = MerchantAccount::unpack(&merchant_info.data.borrow())?;
    if !merchant_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // ensure token accounts are owned by token program
    if *seller_token_info.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    if *buyer_token_info.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Get mint details and verify that they match token account
    let seller_token_data = TokenAccount::unpack(&seller_token_info.data.borrow())?;
    let buyer_token_data = TokenAccount::unpack(&buyer_token_info.data.borrow())?;
    if seller_token_data.mint != buyer_token_data.mint {
        return Err(PaymentProcessorError::MintNotEqual.into());
    }
    // test that order account pubkey is correct
    let address_with_seed = match &order_id.get(..MAX_SEED_LEN) {
        Some(substring) => {
            Pubkey::create_with_seed(signer_info.key, substring, &program_id).unwrap()
        }
        None => Pubkey::create_with_seed(signer_info.key, &order_id, &program_id)
            .unwrap(),
    };
    if *order_info.key != address_with_seed {
        return Err(ProgramError::InvalidSeeds);
    }
    // try create order account
    let order_account_size = get_order_account_size(&order_id);
    let create_account_ix = system_instruction::create_account_with_seed(
        signer_info.key,
        order_info.key,
        signer_info.key,
        &order_id,
        Rent::default().minimum_balance(order_account_size),
        order_account_size as u64,
        program_id,
    );
    msg!("Creating order account on chain...");
    invoke(
        &create_account_ix,
        &[
            signer_info.clone(),
            order_info.clone(),
            signer_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    // ensure merchant account is rent exempt
    if !rent.is_exempt(order_info.lamports(), MerchantAccount::LEN) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    // let payer_token_info = next_account_info(account_info_iter)?;
    // let order_acc_info = next_account_info(account_info_iter)?;
    // let order_token_acc_info = next_account_info(account_info_iter)?;
    // let merchant_acc_info = next_account_info(account_info_iter)?;
    // let mint_acc_info = next_account_info(account_info_iter)?;
    // let token_program_info = next_account_info(account_info_iter)?;
    // let system_program_info = next_account_info(account_info_iter)?;
    // let clock_sysvar_info = next_account_info(account_info_iter)?;
    // let rent_sysvar_info = next_account_info(account_info_iter)?;

    // let rent = &Rent::from_account_info(rent_sysvar_info)?;
    // let timestamp = &Clock::from_account_info(clock_sysvar_info)?.unix_timestamp;

    // // ensure payer token account is owned by token program
    // if *payer_token_info.owner != spl_token::id() {
    //     return Err(ProgramError::IncorrectProgramId);
    // }
    // // ensure merchant account is owned by this program
    // if *merchant_acc_info.owner != *program_id {
    //     return Err(ProgramError::IncorrectProgramId);
    // }
    // // ensure order account is owned by this program
    // if *order_acc_info.owner != *program_id {
    //     return Err(ProgramError::IncorrectProgramId);
    // }
    // // get pda and nonce
    // // let (pda, nonce) = Pubkey::find_program_address(&[b"loan"], program_id);
    // // Get mint details and verify that they match token account
    // let payer_token_account_data = TokenAccount::unpack(&payer_token_info.data.borrow())?;
    // if mint_acc_info.key != &payer_token_account_data.mint {
    //     return Err(ProgramError::IncorrectProgramId);
    // }
    // // let x = spl_token::instruction::initialize_account(
    // //     token_program_info.key,
    // //     &pda,
    // //     &payer_token_account_data.mint,
    // //     program_id
    // // );
    // // let xx = spl_associated_token_account::get_associated_token_address(
    // //     order_acc_info.key,
    // //     &payer_token_account_data.mint
    // // );
    // let create_order_token_acc_ix = spl_associated_token_account::create_associated_token_account(
    //     signer_info.key,
    //     order_acc_info.key,
    //     &payer_token_account_data.mint
    // );
    // invoke(
    //     &create_order_token_acc_ix,
    //     &[
    //         signer_info.clone(),
    //         order_token_acc_info.clone(),
    //         order_acc_info.clone(),
    //         mint_acc_info.clone(),
    //         system_program_info.clone(),
    //         token_program_info.clone(),
    //         rent_sysvar_info.clone(),
    //     ],
    // )?;
    // // Get fee and take home amount
    // let (take_home_amount, fee_amount) = get_amounts(amount);

    // let (pda, nonce) = Pubkey::find_program_address(&[b"payment-processor"], program_id);
    // // get the order account
    // // TODO: ensure this account is not already initialized
    // let mut order_account_data = order_acc_info.try_borrow_mut_data()?;
    // msg!("Saving order information...");
    // let order = OrderAccount {
    //     status: OrderStatus::Paid as u8,
    //     created: *timestamp,
    //     modified: *timestamp,
    //     merchant_pubkey: merchant_acc_info.key.to_bytes(),
    //     mint_pubkey: payer_token_account_data.mint.to_bytes(),
    //     payer_pubkey: signer_info.key.to_bytes(),
    //     expected_amount: amount,
    //     paid_amount: amount,
    //     take_home_amount,
    //     fee_amount: fee_amount as u64,
    //     order_id,
    // };

    // order.pack(&mut order_account_data);

    Ok(())
}
