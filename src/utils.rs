use crate::state::OrderAccount;
use solana_program::pubkey::Pubkey;

/// maximum length of derived `Pubkey` seed
const MAX_SEED_LEN: usize = 32;
/// transaction fee percentage
pub const FEE: u128 = 3;
/// sponsor fee percentage
pub const SPONSOR_FEE: u128 = 3;

/// Given the expected amount, calculate the fee and take home amount
/// Currently fee is 0.3% with a minimum fee of 1 lamport
/// If the amount is less than 100 lamports the fee is 0
pub fn get_amounts(amount: u64, fee_percentage: u128) -> (u64, u64) {
    let mut fee_amount: u64 = 0;
    let mut take_home_amount: u64 = amount;

    if amount >= 100 {
        let possible_fee_amount: u128 = (amount as u128 * fee_percentage) / 1000;
        fee_amount = 1;
        if possible_fee_amount > 0 {
            fee_amount = possible_fee_amount as u64;
        }
        take_home_amount = amount - fee_amount;
    }

    (take_home_amount, fee_amount)
}

/// get order account size
pub fn get_order_account_size(order_id: &String, secret: &String) -> usize {
    return OrderAccount::MIN_LEN + order_id.chars().count() + 4 + secret.chars().count() + 4;
}

// Derive the order account pubkey
pub fn get_order_account_pubkey(
    order_id: &String,
    wallet_pk: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    match &order_id.get(..MAX_SEED_LEN) {
        Some(substring) => Pubkey::create_with_seed(wallet_pk, substring, &program_id).unwrap(),
        None => Pubkey::create_with_seed(wallet_pk, &order_id, &program_id).unwrap(),
    }
}

#[cfg(test)]
mod test {
    use {super::*, solana_program::sysvar, solana_program_test::*, std::str::FromStr};

    #[tokio::test]
    async fn test_get_amounts() {
        assert_eq!((997000000, 3000000), get_amounts(1000000000, FEE));
        assert_eq!((1994000, 6000), get_amounts(2000000, FEE));
        assert_eq!((1994, 6), get_amounts(2000, FEE));
        assert_eq!((100, 1), get_amounts(101, FEE));
        assert_eq!((99, 1), get_amounts(100, FEE));
        assert_eq!((99, 0), get_amounts(99, FEE));
        assert_eq!((80, 0), get_amounts(80, FEE));
        assert_eq!((0, 0), get_amounts(0, FEE));
        assert_eq!((990, 10), get_amounts(1000, 10));
        assert_eq!((996, 4), get_amounts(1000, 4));
    }

    #[tokio::test]
    async fn test_get_order_account_size() {
        assert_eq!(
            199,
            get_order_account_size(&String::from("123456"), &String::from("password"))
        );
        assert_eq!(
            191,
            get_order_account_size(&String::from("test-6"), &String::from(""))
        );
        assert_eq!(424, get_order_account_size(&String::from("WSUDUBDG2"), &String::from("Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type")));
    }

    #[tokio::test]
    async fn test_get_order_account_pubkey() {
        assert_eq!(
            Pubkey::from_str(&"2QaTeJJR9SYvzwZNbRFNpHhQZaxi3o35qb9qJAjBK2Rn").unwrap(),
            get_order_account_pubkey(
                &String::from("123456"),
                &solana_program::system_program::id(),
                &sysvar::clock::id()
            )
        );
    }
}
