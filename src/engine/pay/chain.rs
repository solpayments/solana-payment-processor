use crate::engine::pay::utils::process_order;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use std::collections::BTreeMap;

pub fn process_chain_checkout(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    order_items: BTreeMap<String, u64>,
    maybe_data: Option<String>,
) -> ProgramResult {
    process_order(
        program_id,
        accounts,
        amount,
        format!("{timestamp}", timestamp = Clock::get()?.unix_timestamp),
        "".to_string(),
        maybe_data,
        Some(order_items),
    )?;
    Ok(())
}
