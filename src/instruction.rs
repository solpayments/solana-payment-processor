use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};
use spl_token;

use crate::error::PaymentProcessorError::InvalidInstruction;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
pub enum PaymentProcessorInstruction {
    /// Register for a merchant account.
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person initializing the merchant account
    /// 1. `[writable]` The merchant account.  Owned by this program
    /// 2. `[]` The rent sysvar
    RegisterMerchant,
    /// Express Checkout - create order and pay for it in one transaction
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person initializing the transaction
    /// 1. `[writable]` The payer's token account to be used for the payment
    /// 2. `[writable]` The order account.  Owned by this program
    /// 3. `[writable]` The merchant account.  Owned by this program
    /// 4. `[]` The rent sysvar
    /// 5. `[]` The token program
    ExpressCheckout {
        amount: u64,
        /// the pubkey of the merchant -> this is where the money is to be sent
        /// we are receiving it as data and not an account because during the
        /// express checkout we don't want the UI to have to create this account
        merchant_token_pubkey: [u8; 32],
        /// the external order id (as in issued by the merchant)
        order_id: Vec<u8>,
    },
}

impl PaymentProcessorInstruction {
    /// Unpacks a byte buffer into a [PaymentProcessorInstruction](enum.PaymentProcessorInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, _rest) = input.split_first().ok_or(InvalidInstruction)?;

        Ok(match tag {
            0 => Self::RegisterMerchant,
            _ => return Err(InvalidInstruction.into()),
        })
    }

    pub fn pack_into_vec(&self) -> Vec<u8> {
        self.try_to_vec().expect("try_to_vec")
    }
}

/// Creates an 'RegisterMerchant' instruction.
pub fn register_merchant(
    program_id: Pubkey,
    signer_pubkey: Pubkey,
    merchant_acc_pubkey: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(merchant_acc_pubkey, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: PaymentProcessorInstruction::RegisterMerchant
            .try_to_vec()
            .unwrap(),
    }
}

/// Creates an 'ExpressCheckout' instruction.
pub fn express_checkout(
    program_id: Pubkey,
    signer_pubkey: Pubkey,
    payer_token_acc_pubkey: Pubkey,
    merchant_acc_pubkey: Pubkey,
    amount: u64,
    merchant_token_pubkey: [u8; 32],
    order_id: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(payer_token_acc_pubkey, false),
            AccountMeta::new(merchant_acc_pubkey, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: PaymentProcessorInstruction::ExpressCheckout {
            amount,
            merchant_token_pubkey,
            order_id,
        }
        .pack_into_vec(),
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::processor::process_instruction,
        crate::state::{MerchantAccount, Serdes},
        assert_matches::*,
        solana_program::{rent::Rent, system_instruction},
        solana_program_test::*,
        solana_sdk::{
            signature::{Keypair, Signer},
            transaction::Transaction,
        },
        std::convert::TryInto,
        std::str::FromStr,
    };

    #[tokio::test]
    async fn test_register_merchant() {
        let program_id = Pubkey::from_str(&"mosh111111111111111111111111111111111111111").unwrap();
        let merchant_kepair = Keypair::new();

        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "solana_payment_processor",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        // first create merchant account
        let mut create_user_tx = Transaction::new_with_payer(
            &[system_instruction::create_account(
                &payer.pubkey(),
                &merchant_kepair.pubkey(),
                Rent::default().minimum_balance(MerchantAccount::LEN),
                MerchantAccount::LEN.try_into().unwrap(),
                &program_id,
            )],
            Some(&payer.pubkey()),
        );
        create_user_tx.partial_sign(&[&merchant_kepair], recent_blockhash);
        create_user_tx.sign(&[&payer], recent_blockhash);
        assert_matches!(
            banks_client.process_transaction(create_user_tx).await,
            Ok(())
        );

        // then call register merchant ix
        let mut transaction = Transaction::new_with_payer(
            &[register_merchant(
                program_id,
                payer.pubkey(),
                merchant_kepair.pubkey(),
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));

        // test contents of merchant account
        let merchant_account = banks_client.get_account(merchant_kepair.pubkey()).await;
        let merchant_account = match merchant_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => value,
            },
            Err(error) => panic!("Problem: {:?}", error),
        };
        let merchant_data = MerchantAccount::unpack(&merchant_account.data);
        let merchant_data = match merchant_data {
            Ok(data) => data,
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(true, merchant_data.is_initialized);
        assert_eq!(
            merchant_kepair.pubkey(),
            Pubkey::new_from_array(merchant_data.merchant_pubkey)
        );
        assert_eq!(
            merchant_kepair.pubkey().to_bytes(),
            merchant_data.merchant_pubkey
        );
    }
}
