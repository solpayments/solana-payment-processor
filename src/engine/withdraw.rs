use crate::{
    engine::constants::{PDA_SEED, PROGRAM_OWNER},
    error::PaymentProcessorError,
    state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
    utils::{get_amounts, SPONSOR_FEE},
};
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke_signed},
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    sysvar::{Sysvar},
};
use spl_token::{self, state::Account as TokenAccount};
use std::str::FromStr;

pub fn process_withdraw_payment(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let signer_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let order_payment_token_info = next_account_info(account_info_iter)?;
    let merchant_token_info = next_account_info(account_info_iter)?;
    let program_owner_token_info = next_account_info(account_info_iter)?;
    let sponsor_token_info = next_account_info(account_info_iter)?;
    let pda_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    let timestamp = &Clock::from_account_info(clock_sysvar_info)?.unix_timestamp;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // ensure merchant and order accounts are owned by this program
    if *merchant_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if *order_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    // ensure buyer token account is owned by token program
    if *merchant_token_info.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    // check that provided pda is correct
    let (pda, pda_nonce) = Pubkey::find_program_address(&[PDA_SEED], &program_id);
    if pda_info.key != &pda {
        return Err(ProgramError::InvalidSeeds);
    }
    // check that provided program owner is correct
    let program_owner_token_data = TokenAccount::unpack(&program_owner_token_info.data.borrow())?;
    if program_owner_token_data.owner != Pubkey::from_str(PROGRAM_OWNER).unwrap() {
        return Err(PaymentProcessorError::WrongProgramOwner.into());
    }
    // get the merchant account
    let merchant_account = MerchantAccount::unpack(&merchant_info.data.borrow())?;
    if !merchant_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // ensure that the token account that we will withdraw to is owned by this
    // merchant.  This ensures that anyone can call the withdraw instruction
    // and the money will still go to the right place
    let merchant_token_data = TokenAccount::unpack(&merchant_token_info.data.borrow())?;
    if merchant_token_data.owner != Pubkey::new_from_array(merchant_account.owner_pubkey) {
        return Err(PaymentProcessorError::WrongMerchant.into());
    }
    // check that the provided sponsor is correct
    let sponsor_token_data = TokenAccount::unpack(&sponsor_token_info.data.borrow())?;
    if sponsor_token_data.owner != Pubkey::new_from_array(merchant_account.sponsor_pubkey) {
        return Err(PaymentProcessorError::WrongSponsor.into());
    }
    // get the order account
    let mut order_account = OrderAccount::unpack(&order_info.data.borrow())?;
    if !order_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // ensure order belongs to this merchant
    if merchant_info.key.to_bytes() != order_account.merchant_pubkey {
        return Err(ProgramError::InvalidAccountData);
    }
    // ensure the order payment token account is the right one
    if order_payment_token_info.key.to_bytes() != order_account.token_pubkey {
        return Err(ProgramError::InvalidAccountData);
    }
    // ensure order is not already paid out
    if order_account.status != OrderStatus::Paid as u8 {
        return Err(PaymentProcessorError::AlreadyWithdrawn.into());
    }
    // transfer amount less fees to merchant
    let (associated_token_address, _bump_seed) = Pubkey::find_program_address(
        &[
            &order_info.key.to_bytes(),
            &spl_token::id().to_bytes(),
            &order_account.mint_pubkey,
        ],
        program_id,
    );
    // assert that the derived address matches the one supplied
    if associated_token_address != *order_payment_token_info.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    msg!("Transferring payment to the merchant...");
    invoke_signed(
        &spl_token::instruction::transfer(
            token_program_info.key,
            order_payment_token_info.key,
            merchant_token_info.key,
            &pda,
            &[&pda],
            order_account.paid_amount,
        )
        .unwrap(),
        &[
            token_program_info.clone(),
            pda_info.clone(),
            order_payment_token_info.clone(),
            merchant_token_info.clone(),
        ],
        &[&[&PDA_SEED, &[pda_nonce]]],
    )?;
    if Pubkey::new_from_array(merchant_account.sponsor_pubkey)
        == Pubkey::from_str(PROGRAM_OWNER).unwrap()
    {
        // this means there is no third-party sponsor to pay
        msg!("Transferring processing fee to the program owner...");
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                order_payment_token_info.key,
                program_owner_token_info.key,
                &pda,
                &[&pda],
                order_account.fee_amount,
            )
            .unwrap(),
            &[
                token_program_info.clone(),
                pda_info.clone(),
                order_payment_token_info.clone(),
                program_owner_token_info.clone(),
            ],
            &[&[&PDA_SEED, &[pda_nonce]]],
        )?;
    } else {
        // we need to pay both the program owner and the sponsor
        let (program_owner_fee, sponsor_fee) = get_amounts(order_account.fee_amount, SPONSOR_FEE);
        msg!("Transferring processing fee to the program owner and sponsor...");
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                order_payment_token_info.key,
                program_owner_token_info.key,
                &pda,
                &[&pda],
                program_owner_fee,
            )
            .unwrap(),
            &[
                token_program_info.clone(),
                pda_info.clone(),
                order_payment_token_info.clone(),
                program_owner_token_info.clone(),
            ],
            &[&[&PDA_SEED, &[pda_nonce]]],
        )?;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                order_payment_token_info.key,
                sponsor_token_info.key,
                &pda,
                &[&pda],
                sponsor_fee,
            )
            .unwrap(),
            &[
                token_program_info.clone(),
                pda_info.clone(),
                order_payment_token_info.clone(),
                sponsor_token_info.clone(),
            ],
            &[&[&PDA_SEED, &[pda_nonce]]],
        )?;
    }

    // update the order account data
    msg!("Updating order account information...");
    order_account.status = OrderStatus::Withdrawn as u8;
    order_account.modified = *timestamp;
    OrderAccount::pack(&order_account, &mut order_info.data.borrow_mut());

    Ok(())
}