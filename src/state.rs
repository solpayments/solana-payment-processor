use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::UnixTimestamp,
    program_error::ProgramError,
    pubkey::Pubkey,
    program_pack::{IsInitialized, Pack, Sealed},
};

#[derive(Debug)]
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
    pub mint_pubkey: Pubkey, // represents the token/currency in use
    pub payer_pubkey: Pubkey,
    pub order_id: [u8; 32],
    pub expected_amount: u64,
    pub paid_amount: u64,
    pub fee_amount: u64,
}

// impl for MerchantAccount
impl Sealed for MerchantAccount {}

impl IsInitialized for MerchantAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for MerchantAccount {
    const LEN: usize = 33;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, MerchantAccount::LEN];
        let (
            is_initialized,
            merchant_pubkey,
        ) = array_refs![src, 1, 32];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(MerchantAccount {
            is_initialized,
            merchant_pubkey: Pubkey::new_from_array(*merchant_pubkey),
        })
    }

     fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, MerchantAccount::LEN];
        let (
            is_initialized_dst,
            merchant_pubkey_dst,
        ) = mut_array_refs![dst, 1, 32];

        let MerchantAccount {
            is_initialized,
            merchant_pubkey,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        merchant_pubkey_dst.copy_from_slice(merchant_pubkey.as_ref());
    }
}
