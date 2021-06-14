use crate::{
    engine::constants::{DEFAULT_DATA, PROGRAM_OWNER, SPONSOR_FEE},
    error::PaymentProcessorError,
    state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
    utils::{get_amounts, get_order_account_size},
};
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
use spl_token::{self, state::Account as TokenAccount};
use std::str::FromStr;

pub fn process_express_checkout(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    order_id: String,
    secret: String,
    maybe_data: Option<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let seller_token_info = next_account_info(account_info_iter)?;
    let buyer_token_info = next_account_info(account_info_iter)?;
    let program_owner_info = next_account_info(account_info_iter)?;
    let sponsor_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let pda_info = next_account_info(account_info_iter)?;
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
    // ensure merchant account is owned by this program
    if *merchant_info.owner != *program_id {
        msg!("Error: Wrong owner for merchant account");
        return Err(ProgramError::IncorrectProgramId);
    }
    // get the merchant account
    let merchant_account = MerchantAccount::unpack(&merchant_info.data.borrow())?;
    if !merchant_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // ensure buyer token account is owned by token program
    if *buyer_token_info.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Get mint details and verify that they match token account
    let buyer_token_data = TokenAccount::unpack(&buyer_token_info.data.borrow())?;
    if *mint_info.key != buyer_token_data.mint {
        return Err(PaymentProcessorError::MintNotEqual.into());
    }
    // check that provided program owner is correct
    if *program_owner_info.key != Pubkey::from_str(PROGRAM_OWNER).unwrap() {
        return Err(PaymentProcessorError::WrongProgramOwner.into());
    }
    // check that the provided sponsor is correct
    if *sponsor_info.key != Pubkey::new_from_array(merchant_account.sponsor) {
        msg!("Error: Sponsor account is incorrect");
        return Err(PaymentProcessorError::WrongSponsor.into());
    }
    // create order account
    let data = match maybe_data {
        None => String::from(DEFAULT_DATA),
        Some(value) => value,
    };
    let order_account_size = get_order_account_size(&order_id, &secret, &data);
    // the order account amount includes the fee in SOL
    let order_account_amount = Rent::default().minimum_balance(order_account_size);
    invoke(
        &system_instruction::create_account_with_seed(
            signer_info.key,
            order_info.key,
            signer_info.key,
            &order_id,
            order_account_amount,
            order_account_size as u64,
            program_id,
        ),
        &[
            signer_info.clone(),
            order_info.clone(),
            signer_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    // next we are going to try and create a token account owned by the program
    // but whose address is derived from the order account
    // this is remarkably similar to spl_associated_token_account::create_associated_token_account
    // derive the token account address
    let (associated_token_address, bump_seed) = Pubkey::find_program_address(
        &[
            &order_info.key.to_bytes(),
            &spl_token::id().to_bytes(),
            &mint_info.key.to_bytes(),
        ],
        program_id,
    );
    // assert that the derived address matches the one supplied
    if associated_token_address != *seller_token_info.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }
    // get signer seeds
    let associated_token_account_signer_seeds: &[&[_]] = &[
        &order_info.key.to_bytes(),
        &spl_token::id().to_bytes(),
        &mint_info.key.to_bytes(),
        &[bump_seed],
    ];
    // Fund the associated seller token account with the minimum balance to be rent exempt
    let required_lamports = rent
        .minimum_balance(spl_token::state::Account::LEN)
        .max(1)
        .saturating_sub(seller_token_info.lamports());
    if required_lamports > 0 {
        // Transfer lamports to the associated seller token account
        invoke(
            &system_instruction::transfer(
                &signer_info.key,
                seller_token_info.key,
                required_lamports,
            ),
            &[
                signer_info.clone(),
                seller_token_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }
    // Allocate space for the associated seller token account
    invoke_signed(
        &system_instruction::allocate(seller_token_info.key, spl_token::state::Account::LEN as u64),
        &[seller_token_info.clone(), system_program_info.clone()],
        &[&associated_token_account_signer_seeds],
    )?;
    // Assign the associated seller token account to the SPL Token program
    invoke_signed(
        &system_instruction::assign(seller_token_info.key, &spl_token::id()),
        &[seller_token_info.clone(), system_program_info.clone()],
        &[&associated_token_account_signer_seeds],
    )?;
    // Initialize the associated seller token account
    invoke(
        &spl_token::instruction::initialize_account(
            &spl_token::id(),
            seller_token_info.key,
            mint_info.key,
            pda_info.key,
        )?,
        &[
            seller_token_info.clone(),
            mint_info.clone(),
            pda_info.clone(),
            rent_sysvar_info.clone(),
            token_program_info.clone(),
        ],
    )?;

    // Transfer payment amount to associated seller token account...
    invoke(
        &spl_token::instruction::transfer(
            token_program_info.key,
            buyer_token_info.key,
            seller_token_info.key,
            signer_info.key,
            &[&signer_info.key],
            amount,
        )
        .unwrap(),
        &[
            buyer_token_info.clone(),
            seller_token_info.clone(),
            signer_info.clone(),
            token_program_info.clone(),
        ],
    )?;

    if Pubkey::new_from_array(merchant_account.sponsor) == Pubkey::from_str(PROGRAM_OWNER).unwrap()
    {
        // Transferring processing fee to the program owner...
        invoke(
            &system_instruction::transfer(
                &signer_info.key,
                program_owner_info.key,
                merchant_account.fee,
            ),
            &[
                signer_info.clone(),
                program_owner_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    } else {
        // we need to pay both the program owner and the sponsor
        let (program_owner_fee, sponsor_fee) = get_amounts(merchant_account.fee, SPONSOR_FEE);
        // Transferring processing fee to the program owner and sponsor...
        invoke(
            &system_instruction::transfer(
                &signer_info.key,
                program_owner_info.key,
                program_owner_fee,
            ),
            &[
                signer_info.clone(),
                program_owner_info.clone(),
                system_program_info.clone(),
            ],
        )?;
        invoke(
            &system_instruction::transfer(&signer_info.key, sponsor_info.key, sponsor_fee),
            &[
                signer_info.clone(),
                sponsor_info.clone(),
                system_program_info.clone(),
            ],
        )?;
    }

    // get the order account
    // TODO: ensure this account is not already initialized
    let mut order_account_data = order_info.try_borrow_mut_data()?;
    // Saving order information...
    let order = OrderAccount {
        status: OrderStatus::Paid as u8,
        created: *timestamp,
        modified: *timestamp,
        merchant: merchant_info.key.to_bytes(),
        mint: mint_info.key.to_bytes(),
        token: seller_token_info.key.to_bytes(),
        payer: signer_info.key.to_bytes(),
        expected_amount: amount,
        paid_amount: amount,
        order_id,
        secret,
        data,
    };

    order.pack(&mut order_account_data);

    // ensure order account is rent exempt
    if !rent.is_exempt(order_info.lamports(), order_account_size) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    Ok(())
}
