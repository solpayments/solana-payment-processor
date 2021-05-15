use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::UnixTimestamp,
    program_pack::{IsInitialized, Sealed},
};
use std::mem::size_of;

pub type PublicKey = [u8; 32];

pub trait Serdes: Sized + BorshSerialize + BorshDeserialize {
    fn pack(&self, dst: &mut [u8]) {
        let encoded = self.try_to_vec().unwrap();
        dst[..encoded.len()].copy_from_slice(&encoded);
    }
    fn unpack(src: &[u8]) -> Result<Self, std::io::Error> {
        Self::try_from_slice(src)
    }
}

#[derive(BorshSerialize, BorshSchema, BorshDeserialize, Debug, PartialEq)]
pub struct MerchantAccount {
    pub is_initialized: bool,
    pub owner_pubkey: PublicKey,
    pub sponsor_pubkey: PublicKey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum OrderStatus {
    Uninitialized = 0,
    Pending = 1,
    Paid = 2,
    Withdrawn = 3,
}

#[derive(BorshSerialize, BorshSchema, BorshDeserialize, Debug, PartialEq)]
pub struct OrderAccount {
    pub status: u8,
    pub created: UnixTimestamp,
    pub modified: UnixTimestamp,
    pub merchant_pubkey: PublicKey,
    pub mint_pubkey: PublicKey,  // represents the token/currency in use
    pub token_pubkey: PublicKey, // represents the token account that holds the money
    pub payer_pubkey: PublicKey,
    pub expected_amount: u64,
    pub paid_amount: u64,
    pub fee_amount: u64,
    pub order_id: String,
    pub secret: String,
}

// impl for MerchantAccount
impl Sealed for MerchantAccount {}

impl Serdes for MerchantAccount {}

impl IsInitialized for MerchantAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl MerchantAccount {
    pub const LEN: usize = size_of::<u8>() + size_of::<PublicKey>() + size_of::<PublicKey>();
}

// impl for OrderAccount
impl Sealed for OrderAccount {}

impl Serdes for OrderAccount {}

impl IsInitialized for OrderAccount {
    fn is_initialized(&self) -> bool {
        self.status != OrderStatus::Uninitialized as u8
    }
}

impl OrderAccount {
    pub const MIN_LEN: usize = size_of::<u8>()
        + size_of::<UnixTimestamp>()
        + size_of::<UnixTimestamp>()
        + size_of::<PublicKey>()
        + size_of::<PublicKey>()
        + size_of::<PublicKey>()
        + size_of::<PublicKey>()
        + size_of::<u64>()
        + size_of::<u64>()
        + size_of::<u64>();
}
