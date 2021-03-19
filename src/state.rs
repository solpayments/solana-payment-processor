use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    clock::UnixTimestamp,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
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

#[derive(Debug)]
pub enum OrderStatus {
    Uninitialized = 0,
    Pending = 1,
    Paid = 2,
}

#[derive(Debug)]
pub struct OrderAccount {
    pub status: u8,
    pub created: UnixTimestamp,
    pub modified: UnixTimestamp,
    pub merchant_account: Pubkey,
    pub mint_pubkey: Pubkey, // represents the token/currency in use
    pub payer_pubkey: Pubkey,
    pub order_id: Vec<u8>,
    pub expected_amount: u64,
    pub paid_amount: u64,
    pub fee_amount: u64,
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

// impl Pack for MerchantAccount {
//     const LEN: usize = 33;
//     fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
//         let src = array_ref![src, 0, MerchantAccount::LEN];
//         let (is_initialized, merchant_pubkey) = array_refs![src, 1, 32];
//         let is_initialized = match is_initialized {
//             [0] => false,
//             [1] => true,
//             _ => return Err(ProgramError::InvalidAccountData),
//         };

//         Ok(MerchantAccount {
//             is_initialized,
//             merchant_pubkey: Pubkey::new_from_array(*merchant_pubkey),
//         })
//     }

//     fn pack_into_slice(&self, dst: &mut [u8]) {
//         let dst = array_mut_ref![dst, 0, MerchantAccount::LEN];
//         let (is_initialized_dst, merchant_pubkey_dst) = mut_array_refs![dst, 1, 32];

//         let MerchantAccount {
//             is_initialized,
//             merchant_pubkey,
//         } = self;

//         is_initialized_dst[0] = *is_initialized as u8;
//         merchant_pubkey_dst.copy_from_slice(merchant_pubkey.as_ref());
//     }
// }

// impl for MerchantAccount
impl Sealed for OrderAccount {}

impl IsInitialized for OrderAccount {
    fn is_initialized(&self) -> bool {
        self.status != OrderStatus::Uninitialized as u8
    }
}

// impl Pack for OrderAccount {
//     const LEN: usize = 145;
//     fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
//         let src = array_ref![src, 0, OrderAccount::LEN];
//         let (
//             status,
//             created,
//             modified,
//             merchant_account,
//             mint_pubkey,
//             payer_pubkey,
//             order_id,
//             expected_amount,
//             paid_amount,
//             fee_amount,
//         ) = array_refs![src, 1, 8, 8, 32, 32, 32, 8, 8, 8, 8];

//         Ok(OrderAccount {
//             status: u8::from_le_bytes(*status),
//             created: i64::from_le_bytes(*created),
//             modified: i64::from_le_bytes(*modified),
//             merchant_account: Pubkey::new_from_array(*merchant_account),
//             mint_pubkey: Pubkey::new_from_array(*mint_pubkey),
//             payer_pubkey: Pubkey::new_from_array(*payer_pubkey),
//             order_id: Vec::<u8>::from(*order_id),
//             expected_amount: u64::from_le_bytes(*expected_amount),
//             paid_amount: u64::from_le_bytes(*paid_amount),
//             fee_amount: u64::from_le_bytes(*fee_amount),
//         })
//     }

//     fn pack_into_slice(&self, dst: &mut [u8]) {
//         let dst = array_mut_ref![dst, 0, OrderAccount::LEN];
//         let (
//             status_dst,
//             created_dst,
//             modified_dst,
//             merchant_account_dst,
//             mint_pubkey_dst,
//             payer_pubkey_dst,
//             order_id_dst,
//             expected_amount_dst,
//             paid_amount_dst,
//             fee_amount_dst,
//         ) = mut_array_refs![dst, 1, 8, 8, 32, 32, 32, 8, 8, 8, 8];

//         let OrderAccount {
//             status,
//             created,
//             modified,
//             merchant_account,
//             mint_pubkey,
//             payer_pubkey,
//             order_id,
//             expected_amount,
//             paid_amount,
//             fee_amount,
//         } = self;

//         *status_dst = status.to_le_bytes();
//         *created_dst = created.to_le_bytes();
//         *modified_dst = modified.to_le_bytes();
//         merchant_account_dst.copy_from_slice(merchant_account.as_ref());
//         mint_pubkey_dst.copy_from_slice(mint_pubkey.as_ref());
//         payer_pubkey_dst.copy_from_slice(payer_pubkey.as_ref());
//         *order_id_dst = order_id.as_slice();
//         *expected_amount_dst = expected_amount.to_le_bytes();
//         *paid_amount_dst = paid_amount.to_le_bytes();
//         *fee_amount_dst = fee_amount.to_le_bytes();
//     }
// }
