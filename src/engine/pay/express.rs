use crate::engine::pay::utils::process_order;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

pub fn process_express_checkout(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    order_id: String,
    secret: String,
    maybe_data: Option<String>,
) -> ProgramResult {
    process_order(
        program_id,
        accounts,
        amount,
        order_id,
        secret,
        maybe_data,
        Option::None,
    )?;
    Ok(())
}
