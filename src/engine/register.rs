use crate::{
    engine::constants::{DEFAULT_DATA, MERCHANT, MIN_FEE_IN_LAMPORTS, PROGRAM_OWNER},
    state::{MerchantAccount, MerchantStatus, Serdes},
    utils::get_merchant_account_size,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use std::str::FromStr;

pub fn process_register_merchant(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    seed: Option<String>,
    maybe_fee: Option<u64>,
    maybe_data: Option<String>,
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

    let data = match maybe_data {
        None => String::from(DEFAULT_DATA),
        Some(value) => value,
    };
    let account_size = get_merchant_account_size(&data);

    // Creating merchant account on chain...
    invoke(
        &system_instruction::create_account_with_seed(
            signer_info.key,
            merchant_info.key,
            signer_info.key,
            match &seed {
                None => MERCHANT,
                Some(value) => &value,
            },
            Rent::default().minimum_balance(account_size),
            account_size as u64,
            program_id,
        ),
        &[
            signer_info.clone(),
            merchant_info.clone(),
            signer_info.clone(),
            system_sysvar_info.clone(),
        ],
    )?;

    // get the merchant account data
    // TODO: ensure this account is not already initialized
    let mut merchant_account_data = merchant_info.try_borrow_mut_data()?;
    // save it
    let merchant = MerchantAccount {
        status: MerchantStatus::Initialized as u8,
        owner: signer_info.key.to_bytes(),
        sponsor: match possible_sponsor_info {
            Ok(sponsor_info) => sponsor_info.key.to_bytes(),
            Err(_error) => Pubkey::from_str(PROGRAM_OWNER).unwrap().to_bytes(),
        },
        fee: match maybe_fee {
            None => MIN_FEE_IN_LAMPORTS,
            Some(value) => {
                let mut result = MIN_FEE_IN_LAMPORTS;
                if value > MIN_FEE_IN_LAMPORTS {
                    result = value;
                }
                result
            }
        },
        data,
    };

    merchant.pack(&mut merchant_account_data);

    // ensure merchant account is rent exempt
    if !rent.is_exempt(merchant_info.lamports(), account_size) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    Ok(())
}
