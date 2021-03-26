const FEE: u64 = 3;

/// Given the expected amount, calculate the fee and take home amount
/// Currently fee is 0.3% with a minimum fee of 1 lamport
pub fn get_amounts(amount: u64) -> (u64, u64) {
    let possible_fee_amount: u128 = (amount as u128 * FEE as u128) / 1000;
    let mut fee_amount: u64 = 1;
    if possible_fee_amount > 0 {
        fee_amount = possible_fee_amount as u64;
    }
    let take_home_amount = amount - fee_amount as u64;

    (take_home_amount, fee_amount)
}
