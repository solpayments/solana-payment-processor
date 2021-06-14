/// the word merchant as a string
pub const MERCHANT: &str = "merchant";
/// seed for pgram derived addresses
pub const PDA_SEED: &[u8] = b"sol_payment_processor";
/// the program owner
pub const PROGRAM_OWNER: &str = "mosh782eoKyPca9eotWfepHVSKavjDMBjNkNE3Gge6Z";
/// maximum length of derived `Pubkey` seed
pub const MAX_SEED_LEN: usize = 32;
/// minimum transaction fee percentage
pub const MIN_FEE_IN_LAMPORTS: u64 = 5000;
/// sponsor fee percentage
pub const SPONSOR_FEE: u128 = 3;
/// default data value
pub const DEFAULT_DATA: &str = "{}";
// these are purely by trial and error ... TODO: understand these some more
/// the mem size of string ... apparently
pub const STRING_SIZE: usize = 4;
