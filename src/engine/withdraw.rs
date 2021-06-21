use crate::{
    engine::common::{get_subscription_package, verify_subscription_order},
    engine::constants::{PACKAGES, PDA_SEED, TRIAL},
    error::PaymentProcessorError,
    state::{MerchantAccount, OrderAccount, OrderStatus, Serdes, SubscriptionAccount},
};
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use spl_token::{self, state::Account as TokenAccount};

pub fn process_withdraw_payment(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let signer_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let order_payment_token_info = next_account_info(account_info_iter)?;
    let merchant_token_info = next_account_info(account_info_iter)?;
    let pda_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let timestamp = Clock::get()?.unix_timestamp;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // ensure merchant and order accounts are owned by this program
    if *merchant_info.owner != *program_id {
        msg!("Error: Wrong owner for merchant account");
        return Err(ProgramError::IncorrectProgramId);
    }
    if *order_info.owner != *program_id {
        msg!("Error: Wrong owner for order account");
        return Err(ProgramError::IncorrectProgramId);
    }
    // ensure buyer token account is owned by token program
    if *merchant_token_info.owner != spl_token::id() {
        msg!("Error: Token account must be owned by token program");
        return Err(ProgramError::IncorrectProgramId);
    }
    // check that provided pda is correct
    let (pda, pda_nonce) = Pubkey::find_program_address(&[PDA_SEED], &program_id);
    if pda_info.key != &pda {
        return Err(ProgramError::InvalidSeeds);
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
    if merchant_token_data.owner != Pubkey::new_from_array(merchant_account.owner) {
        return Err(PaymentProcessorError::WrongMerchant.into());
    }
    // get the order account
    let mut order_account = OrderAccount::unpack(&order_info.data.borrow())?;
    if !order_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    // ensure order belongs to this merchant
    if merchant_info.key.to_bytes() != order_account.merchant {
        return Err(ProgramError::InvalidAccountData);
    }
    // ensure the order payment token account is the right one
    if order_payment_token_info.key.to_bytes() != order_account.token {
        return Err(ProgramError::InvalidAccountData);
    }
    // ensure order is not already paid out
    if order_account.status != OrderStatus::Paid as u8 {
        return Err(PaymentProcessorError::AlreadyWithdrawn.into());
    }
    // check if this is for a subscription payment that has a trial period
    if merchant_account.data.contains(PACKAGES) && merchant_account.data.contains(TRIAL) {
        let subscription_info = next_account_info(account_info_iter)?;
        // ensure subscription account is owned by this program
        if *subscription_info.owner != *program_id {
            msg!("Error: Wrong owner for subscription account");
            return Err(ProgramError::IncorrectProgramId);
        }
        // ensure this order is for this subscription
        verify_subscription_order(subscription_info, &order_account)?;
        // get the subscription account
        let subscription_account = SubscriptionAccount::unpack(&subscription_info.data.borrow())?;
        if !subscription_account.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }
        let package = get_subscription_package(&subscription_account.name, &merchant_account)?;
        // get the trial period duration
        let trial_duration: i64 = match package.trial {
            None => 0,
            Some(value) => value,
        };
        // don't allow withdrawal if still within trial period
        if timestamp < (subscription_account.joined + trial_duration) {
            return Err(PaymentProcessorError::CantWithdrawDuringTrial.into());
        }
    }
    // Transferring payment to the merchant...
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

    // Updating order account information...
    order_account.status = OrderStatus::Withdrawn as u8;
    order_account.modified = timestamp;
    OrderAccount::pack(&order_account, &mut order_info.data.borrow_mut());

    Ok(())
}
