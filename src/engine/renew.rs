use crate::engine::json::{OrderSubscription, Packages};
use crate::error::PaymentProcessorError;
use crate::state::{MerchantAccount, OrderAccount, OrderStatus, Serdes, SubscriptionAccount};
use serde_json::Error as JSONError;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

pub fn process_renew_subscription(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    quantity: i64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let subscription_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // ensure merchant & order & subscription accounts are owned by this program
    if *merchant_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if *order_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if *subscription_info.owner != *program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    // get the merchant account
    let merchant_account = MerchantAccount::unpack(&merchant_info.data.borrow())?;
    if !merchant_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // get the order account
    let order_account = OrderAccount::unpack(&order_info.data.borrow())?;
    // ensure this order is for this subscription
    let order_json_data: Result<OrderSubscription, JSONError> =
        serde_json::from_str(&order_account.data);
    let expected_subscription = match order_json_data {
        Err(_error) => return Err(PaymentProcessorError::InvalidSubscriptionData.into()),
        Ok(data) => data.subscription,
    };
    if expected_subscription != subscription_info.key.to_string() {
        return Err(PaymentProcessorError::WrongOrderAccount.into());
    }
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
    // get the subscription account
    let mut subscription_account = SubscriptionAccount::unpack(&subscription_info.data.borrow())?;
    if !subscription_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // ensure the merchant has a subscription by this name
    let name_vec: Vec<&str> = subscription_account.name.split(":").collect();
    let package_name = name_vec[1];
    let merchant_json_data: Result<Packages, JSONError> =
        serde_json::from_str(&merchant_account.data);
    let packages = match merchant_json_data {
        Err(_error) => return Err(PaymentProcessorError::InvalidSubscriptionData.into()),
        Ok(data) => data.packages,
    };
    // WARNING: if more than one sub of the same name is found, take the first one
    // TODO: verify ^^
    let package = packages
        .into_iter()
        .find(|package| package.name == package_name);
    let package = match package {
        None => return Err(PaymentProcessorError::InvalidSubscriptionPackage.into()),
        Some(value) => value,
    };
    // ensure the amount paid is as expected
    let expected_amount = (quantity as u64) * package.price;
    if expected_amount > order_account.paid_amount {
        return Err(PaymentProcessorError::NotFullyPaid.into());
    }
    // update subscription account
    let timestamp = &Clock::from_account_info(clock_sysvar_info)?.unix_timestamp;
    if timestamp > &subscription_account.period_end {
        // had ended so we start a new period
        subscription_account.period_start = *timestamp;
        subscription_account.period_end = *timestamp + (package.duration * quantity);
    } else {
        // not yet ended so we add the time to the end of the current period
        subscription_account.period_end =
            subscription_account.period_end + (package.duration * quantity);
    }
    SubscriptionAccount::pack(
        &subscription_account,
        &mut subscription_info.data.borrow_mut(),
    );

    Ok(())
}
