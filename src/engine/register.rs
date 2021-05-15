use crate::{
    engine::constants::{MERCHANT, PROGRAM_OWNER},
    state::{MerchantAccount, Serdes},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke},
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use std::convert::TryInto;
use std::str::FromStr;

pub fn process_register_merchant(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    seed: Option<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let system_sysvar_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    let possible_sponsor_info = next_account_info(account_info_iter);
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // create merchant account
    msg!("Creating merchant account on chain...");
    invoke(
        &system_instruction::create_account_with_seed(
            signer_info.key,
            merchant_info.key,
            signer_info.key,
            match &seed {
                None => MERCHANT,
                Some(value) => &value,
            },
            Rent::default().minimum_balance(MerchantAccount::LEN),
            MerchantAccount::LEN.try_into().unwrap(),
            program_id,
        ),
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
    merchant_account.is_initialized = true;
    merchant_account.owner_pubkey = signer_info.key.to_bytes();
    // set the sponsor as provided or default to the program owner
    merchant_account.sponsor_pubkey = match possible_sponsor_info {
        Ok(sponsor_info) => sponsor_info.key.to_bytes(),
        Err(_error) => Pubkey::from_str(PROGRAM_OWNER).unwrap().to_bytes(),
    };
    MerchantAccount::pack(&merchant_account, &mut merchant_info.data.borrow_mut());

    Ok(())
}