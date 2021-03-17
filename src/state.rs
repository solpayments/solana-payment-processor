use solana_sdk::{ clock::UnixTimestamp };
use solana_program::{ pubkey::Pubkey };

pub struct MerchantAccount {
    pub is_initialized: bool,
    pub merchant_pubkey: Pubkey,
}

pub enum OrderStatus {
    Pending = 0,
    Paid = 1,
}

pub struct OrderAccount {
    pub status: u8,
    pub created: UnixTimestamp,
    pub modified: UnixTimestamp,
    pub merchant_account: Pubkey,
    pub payer_pubkey: Pubkey,
    pub order_id: [u8; 32],
    pub expected_amount: u64,
    pub paid_amount: u64,
    pub fee_amount: u64,
}