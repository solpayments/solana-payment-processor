use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    clock::UnixTimestamp,
    program_error::ProgramError,
    program_pack::{IsInitialized, Sealed},
};

pub type PublicKey = [u8; 32];
pub trait Serdes: Sized + BorshSerialize + BorshDeserialize {
    fn pack(&self, dst: &mut [u8]) {
        let encoded = self.try_to_vec().unwrap();
        dst[..encoded.len()].copy_from_slice(&encoded);
    }
    fn unpack(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| ProgramError::InvalidAccountData)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct MerchantAccount {
    pub is_initialized: bool,
    pub merchant_pubkey: PublicKey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum OrderStatus {
    Uninitialized = 0,
    Pending = 1,
    Paid = 2,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct OrderAccount {
    pub status: u8,
    pub created: UnixTimestamp,
    pub modified: UnixTimestamp,
    pub merchant_pubkey: PublicKey,
    pub mint_pubkey: PublicKey, // represents the token/currency in use
    pub payer_pubkey: PublicKey,
    pub expected_amount: u64,
    pub paid_amount: u64,
    pub fee_amount: u64,
    pub order_id: Vec<u8>,  // size of this turns out to be 4
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
    pub const LEN: usize = 33;
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
    pub const LEN: usize = 141;
}
