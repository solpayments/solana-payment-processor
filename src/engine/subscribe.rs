use crate::engine::common::subscribe_checks;
use crate::error::PaymentProcessorError;
use crate::state::{Serdes, SubscriptionAccount, SubscriptionStatus};
use crate::utils::get_subscription_account_size;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

pub fn process_subscribe(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    maybe_data: Option<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let subscription_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;
    let rent_sysvar_info = next_account_info(account_info_iter)?;

    let (order_account, package) = subscribe_checks(
        program_id,
        signer_info,
        merchant_info,
        order_info,
        subscription_info,
        &name,
    )?;

    // ensure the amount paid is as expected
    if package.price > order_account.paid_amount {
        return Err(PaymentProcessorError::NotFullyPaid.into());
    }
    // get subscription account size
    let data = match maybe_data {
        None => String::from("{}"),
        Some(value) => value,
    };
    let account_size = get_subscription_account_size(&name, &data);
    // Creating subscription account on chain...
    invoke(
        &system_instruction::create_account_with_seed(
            signer_info.key,
            subscription_info.key,
            signer_info.key,
            &name,
            Rent::default().minimum_balance(account_size),
            account_size as u64,
            program_id,
        ),
        &[
            signer_info.clone(),
            subscription_info.clone(),
            signer_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    let rent = &Rent::from_account_info(rent_sysvar_info)?;
    let timestamp = &Clock::from_account_info(clock_sysvar_info)?.unix_timestamp;

    // get the subscription account
    // TODO: ensure this account is not already initialized
    let mut subscription_data = subscription_info.try_borrow_mut_data()?;
    // Saving subscription information...
    let subscription = SubscriptionAccount {
        status: SubscriptionStatus::Initialized as u8,
        owner: signer_info.key.to_bytes(),
        merchant: merchant_info.key.to_bytes(),
        name,
        joined: *timestamp,
        period_start: *timestamp,
        period_end: *timestamp + package.duration,
        data,
    };
    subscription.pack(&mut subscription_data);

    // ensure subscription account is rent exempt
    if !rent.is_exempt(subscription_info.lamports(), account_size) {
        return Err(ProgramError::AccountNotRentExempt);
    }

    Ok(())
}
