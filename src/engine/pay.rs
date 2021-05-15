use crate::{
    error::PaymentProcessorError,
    state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
    utils::{get_amounts, get_order_account_size, FEE},
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

pub fn process_express_checkout(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    order_id: String,
    secret: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let seller_token_info = next_account_info(account_info_iter)?;
    let buyer_token_info = next_account_info(account_info_iter)?;
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
    // create order account
    let order_account_size = get_order_account_size(&order_id, &secret);
    msg!("Creating order account on chain...");
    invoke(
        &system_instruction::create_account_with_seed(
            signer_info.key,
            order_info.key,
            signer_info.key,
            &order_id,
            Rent::default().minimum_balance(order_account_size),
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

    // next we are going to try and create a token account owned by the order
    // account and whose address is 'owned' by the order account
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
        msg!(
            "Transfer {} lamports to the associated seller token account",
            required_lamports
        );
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
    msg!("Allocate space for the associated seller token account");
    invoke_signed(
        &system_instruction::allocate(seller_token_info.key, spl_token::state::Account::LEN as u64),
        &[seller_token_info.clone(), system_program_info.clone()],
        &[&associated_token_account_signer_seeds],
    )?;
    msg!("Assign the associated seller token account to the SPL Token program");
    invoke_signed(
        &system_instruction::assign(seller_token_info.key, &spl_token::id()),
        &[seller_token_info.clone(), system_program_info.clone()],
        &[&associated_token_account_signer_seeds],
    )?;
    msg!("Initialize the associated seller token account");
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

    msg!("Transfer payment amount to associated seller token account...");
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

    // Get fee and take home amount
    let (take_home_amount, fee_amount) = get_amounts(amount, FEE);

    // get the order account
    // TODO: ensure this account is not already initialized
    let mut order_account_data = order_info.try_borrow_mut_data()?;
    msg!("Saving order information...");
    let order = OrderAccount {
        status: OrderStatus::Paid as u8,
        created: *timestamp,
        modified: *timestamp,
        merchant_pubkey: merchant_info.key.to_bytes(),
        mint_pubkey: mint_info.key.to_bytes(),
        token_pubkey: seller_token_info.key.to_bytes(),
        payer_pubkey: signer_info.key.to_bytes(),
        expected_amount: amount,
        paid_amount: amount,
        take_home_amount,
        fee_amount: fee_amount as u64,
        order_id,
        secret,
    };

    order.pack(&mut order_account_data);

    // ensure order account is rent exempt
    if !rent.is_exempt(order_info.lamports(), order_account_size) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    Ok(())
}