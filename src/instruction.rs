use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};
use spl_token::{self};

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
    /// 4. `[]` The token program
    /// 5. `[]` The clock sysvar
    /// 6. `[]` The rent sysvar
    ExpressCheckout {
        amount: u64,
        /// the pubkey of the merchant -> this is where the money is to be sent
        /// we are receiving it as data and not an account because during the
        /// express checkout we don't want the UI to have to create this account
        // merchant_token_pubkey: [u8; 32],
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
    order_acc_pubkey: Pubkey,
    merchant_acc_pubkey: Pubkey,
    amount: u64,
    order_id: Vec<u8>,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new(payer_token_acc_pubkey, false),
            AccountMeta::new(order_acc_pubkey, false),
            AccountMeta::new_readonly(merchant_acc_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: PaymentProcessorInstruction::ExpressCheckout {
            amount,
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
        crate::state::{MerchantAccount, OrderAccount, Serdes},
        assert_matches::*,
        solana_program::{hash::Hash, rent::Rent, system_instruction},
        solana_program_test::*,
        solana_sdk::{
            signature::{Keypair, Signer},
            transaction::Transaction,
        },
        spl_token::instruction::initialize_account,
        std::convert::TryInto,
        std::str::FromStr,
    };

    fn create_token_account(
        payer: &Keypair,
        mint: &Keypair,
        recent_blockhash: Hash,
        token_account: &Keypair,
        token_account_owner: &Pubkey,
    ) -> Transaction {
        let instructions = [
            system_instruction::create_account(
                &payer.pubkey(),
                &token_account.pubkey(),
                Rent::default().minimum_balance(165),
                165,
                &spl_token::id(),
            ),
            initialize_account(
                &spl_token::id(),
                &token_account.pubkey(),
                &mint.pubkey(),
                token_account_owner,
            )
            .unwrap(),
        ];
        let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
        transaction.partial_sign(&[payer, token_account], recent_blockhash);
        transaction
    }

    async fn create_merchant_account() -> (Pubkey, Keypair, BanksClient, Keypair, Hash) {
        let program_id = Pubkey::from_str(&"mosh111111111111111111111111111111111111111").unwrap();
        let merchant_keypair = Keypair::new();

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
                &merchant_keypair.pubkey(),
                Rent::default().minimum_balance(MerchantAccount::LEN),
                MerchantAccount::LEN.try_into().unwrap(),
                &program_id,
            )],
            Some(&payer.pubkey()),
        );
        create_user_tx.partial_sign(&[&merchant_keypair], recent_blockhash);
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
                merchant_keypair.pubkey(),
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
        return (
            program_id,
            merchant_keypair,
            banks_client,
            payer,
            recent_blockhash,
        );
    }

    #[tokio::test]
    async fn test_register_merchant() {
        let result = create_merchant_account().await;
        let merchant_keypair = result.1;
        let mut banks_client = result.2;

        // test contents of merchant account
        let merchant_account = banks_client.get_account(merchant_keypair.pubkey()).await;
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
            merchant_keypair.pubkey(),
            Pubkey::new_from_array(merchant_data.merchant_pubkey)
        );
        assert_eq!(
            merchant_keypair.pubkey().to_bytes(),
            merchant_data.merchant_pubkey
        );
    }

    #[tokio::test]
    async fn test_express_checkout() {
        let order_keypair = Keypair::new();
        let result = create_merchant_account().await;
        let program_id = result.0;
        let merchant_keypair = result.1;
        let mut banks_client = result.2;
        let payer = result.3;
        let recent_blockhash = result.4;

        // first create order account
        let mut create_order_tx = Transaction::new_with_payer(
            &[system_instruction::create_account(
                &payer.pubkey(),
                &order_keypair.pubkey(),
                Rent::default().minimum_balance(OrderAccount::LEN),
                OrderAccount::LEN.try_into().unwrap(),
                &program_id,
            )],
            Some(&payer.pubkey()),
        );
        create_order_tx.partial_sign(&[&order_keypair], recent_blockhash);
        create_order_tx.sign(&[&payer], recent_blockhash);
        assert_matches!(
            banks_client.process_transaction(create_order_tx).await,
            Ok(())
        );
        // next create token account for test
        let mint_keypair = Keypair::new();
        let token_keypair = Keypair::new();
        let token_tx = create_token_account(
            &payer,
            &mint_keypair,
            recent_blockhash,
            &token_keypair,
            &payer.pubkey(),
        );
        assert_matches!(
            banks_client.process_transaction(token_tx).await,
            Ok(())
        );
        // then call express checkout ix
        let mut transaction = Transaction::new_with_payer(
            &[express_checkout(
                program_id,
                payer.pubkey(),
                token_keypair.pubkey(),
                order_keypair.pubkey(),
                merchant_keypair.pubkey(),
                2000,
                String::from("#123").into_bytes(),
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
    }
}
