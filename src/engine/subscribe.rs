use crate::engine::json::{Packages};
use crate::error::PaymentProcessorError;
use crate::state::{
    MerchantAccount, OrderAccount, OrderStatus, Serdes, SubscriptionAccount, SubscriptionStatus,
};
use crate::utils::get_subscription_account_size;
use serde_json::Error as JSONError;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    program_pack::IsInitialized,
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

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // ensure merchant & order accounts are owned by this program
    if *merchant_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if *order_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    // get the merchant account
    let merchant_account = MerchantAccount::unpack(&merchant_info.data.borrow())?;
    if !merchant_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // get the order account
    let order_account = OrderAccount::unpack(&order_info.data.borrow())?;
    // ensure we have the right payer
    if signer_info.key.to_bytes() != order_account.payer {
        return Err(PaymentProcessorError::WrongPayer.into());
    }
    // ensure order account is paid
    if order_account.status != (OrderStatus::Paid as u8) {
        return Err(PaymentProcessorError::NotPaid.into());
    }
    // ensure the order account belongs to this merchant
    if merchant_info.key.to_bytes() != order_account.merchant {
        return Err(ProgramError::InvalidAccountData);
    }
    // ensure the order id is this subscription name
    // TODO: what order id to use for paying for the same subscription?
    let name_vec: Vec<&str> = name.split(":").collect();
    let package_name = name_vec[1];
    if order_account.order_id != package_name {
        return Err(PaymentProcessorError::InvalidOrder.into());
    }
    // ensure the merchant has a subscription by this name
    let json_data: Result<Packages, JSONError> = serde_json::from_str(&merchant_account.data);
    let packages = match json_data {
        Err(_error) => return Err(PaymentProcessorError::InvalidSubscriptionData.into()),
        Ok(data) => data.packages,
    };
    // TODO: what happens when more than one subscription of same name exists?
    let package = packages
        .into_iter()
        .find(|package| package.name == package_name);
    let package = match package {
        None => return Err(PaymentProcessorError::InvalidSubscriptionPackage.into()),
        Some(value) => value,
    };
    // ensure the amount paid is as expected
    // TODO: this is wrong it should be duration * number of periods
    if (package.price * (package.duration as u64)) > order_account.paid_amount {
        return Err(PaymentProcessorError::NotFullyPaid.into());
    }
    // get subscription account size
    let data = match maybe_data {
        None => String::from("{}"),
        Some(value) => value,
    };
    let account_size = get_subscription_account_size(&name, &data);
    // TODO: prevent one order account from being used for more than one subscription
    msg!("Creating subscription account on chain...");
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
    msg!("Saving subscription information...");
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
