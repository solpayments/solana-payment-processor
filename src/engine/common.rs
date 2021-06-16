use crate::engine::json::{OrderSubscription, Package, Packages};
use crate::error::PaymentProcessorError;
use crate::state::{MerchantAccount, OrderAccount, OrderStatus, Serdes};
use serde_json::Error as JSONError;
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};
use std::io::Cursor;
use murmur3::murmur3_32;

pub fn subscribe_checks(
    program_id: &Pubkey,
    signer_info: &AccountInfo<'_>,
    merchant_info: &AccountInfo<'_>,
    order_info: &AccountInfo<'_>,
    subscription_info: &AccountInfo<'_>,
    subscription_name: &str,
) -> Result<(OrderAccount, Package), ProgramError> {
    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // ensure merchant & order accounts are owned by this program
    if *merchant_info.owner != *program_id {
        msg!("Error: Wrong owner for merchant account");
        return Err(ProgramError::IncorrectProgramId);
    }
    if *order_info.owner != *program_id {
        msg!("Error: Wrong owner for order account");
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
    // ensure the merchant has a subscription by this name
    // TODO: using split looks janky.  Is it necessary?
    let name_vec: Vec<&str> = subscription_name.split(":").collect();
    let package_name = name_vec[1];
    let merchant_json_data: Result<Packages, JSONError> =
        serde_json::from_str(&merchant_account.data);
    let packages = match merchant_json_data {
        Err(_error) => return Err(PaymentProcessorError::InvalidSubscriptionData.into()),
        Ok(data) => data.packages,
    };
    // NB: if the are duplicates, take the first one --> verified in a test
    let package = packages
        .into_iter()
        .find(|package| package.name == package_name);
    let package = match package {
        None => return Err(PaymentProcessorError::InvalidSubscriptionPackage.into()),
        Some(value) => value,
    };
    if package.mint != Pubkey::new_from_array(order_account.mint).to_string() {
        return Err(PaymentProcessorError::WrongMint.into());
    }
    Ok((order_account, package))
}

/// Get hash of a string
///
/// We are using murmur3 as the hashing algorithm as we don't need a
/// cryptographically secure hashing algorithm.  We mostly need something fast
/// with reasonably low chances of collisions
pub fn hash(input: &str) -> String {
    format!("{}", murmur3_32(&mut Cursor::new(input), 0).unwrap())
}

/// Get a hashed seed phrase
///
/// Basically hashes a base public key concatenated with an input string
pub fn get_hashed_seed(base: &Pubkey, input: &str) -> String {
    hash(&format!("{}:{}", base.to_string(), input))
}
