use crate::{
    engine::common::subscribe_checks,
    engine::constants::PDA_SEED,
    state::{OrderAccount, OrderStatus, Serdes, SubscriptionAccount},
};
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
use spl_token::{self};

pub fn process_cancel_subscription(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let signer_info = next_account_info(account_info_iter)?;
    let subscription_info = next_account_info(account_info_iter)?;
    let merchant_info = next_account_info(account_info_iter)?;
    let order_info = next_account_info(account_info_iter)?;
    let order_token_info = next_account_info(account_info_iter)?;
    let refund_token_info = next_account_info(account_info_iter)?;
    let pda_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let timestamp = Clock::get()?.unix_timestamp;

    // ensure signer can sign
    if !signer_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // ensure subscription account is owned by this program
    if *subscription_info.owner != *program_id {
        msg!("Error: Wrong owner for subscription account");
        return Err(ProgramError::IncorrectProgramId);
    }
    // ensure token accounts are owned by token program
    if *order_token_info.owner != spl_token::id() {
        msg!("Error: Order token account must be owned by token program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if *refund_token_info.owner != spl_token::id() {
        msg!("Error: Refund token account must be owned by token program");
        return Err(ProgramError::IncorrectProgramId);
    }
    // check that provided pda is correct
    let (pda, pda_nonce) = Pubkey::find_program_address(&[PDA_SEED], &program_id);
    if pda_info.key != &pda {
        return Err(ProgramError::InvalidSeeds);
    }

    // get the subscription account
    let subscription_account = SubscriptionAccount::unpack(&subscription_info.data.borrow())?;
    if !subscription_account.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }
    let (mut order_account, package) = subscribe_checks(
        program_id,
        signer_info,
        merchant_info,
        order_info,
        subscription_info,
        &subscription_account.name,
    )?;

    // ensure the order payment token account is the right one
    if order_token_info.key.to_bytes() != order_account.token {
        msg!("Error: Incorrect order token account");
        return Err(ProgramError::InvalidAccountData);
    }
    // ensure the signer is the order payer
    if signer_info.key.to_bytes() != order_account.payer {
        msg!("Error: One can only cancel their own subscription payment");
        return Err(ProgramError::InvalidAccountData);
    }

    // get the trial period duration
    let trial_duration: i64 = match package.trial {
        None => 0,
        Some(value) => value,
    };
    // don't allow cancellation if trial period ended
    if timestamp > (subscription_account.joined + trial_duration) {
        msg!("Info: Subscription amount not refunded because trial period has ended.");
    } else {
        // Transferring payment back to the payer...
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                order_token_info.key,
                refund_token_info.key,
                &pda,
                &[&pda],
                order_account.paid_amount,
            )
            .unwrap(),
            &[
                token_program_info.clone(),
                pda_info.clone(),
                order_token_info.clone(),
                refund_token_info.clone(),
            ],
            &[&[&PDA_SEED, &[pda_nonce]]],
        )?;
    }

    // Updating order account information...
    order_account.status = OrderStatus::Cancelled as u8;
    order_account.modified = timestamp;
    OrderAccount::pack(&order_account, &mut order_info.data.borrow_mut());

    Ok(())
}
