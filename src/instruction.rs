use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};
use spl_token::{self};

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
        #[allow(dead_code)] // not dead code..
        amount: u64,
        /// the pubkey of the merchant -> this is where the money is to be sent
        /// we are receiving it as data and not an account because during the
        /// express checkout we don't want the UI to have to create this account
        // merchant_token_pubkey: [u8; 32],
        /// the external order id (as in issued by the merchant)
        #[allow(dead_code)] // not dead code..
        order_id: String,
    },
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
    order_id: String,
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
        data: PaymentProcessorInstruction::ExpressCheckout { amount, order_id }
            .try_to_vec()
            .unwrap(),
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        // crate::processor::process_instruction,
        crate::instruction::PaymentProcessorInstruction,
        crate::state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
        assert_matches::*,
        solana_program::{
            hash::Hash,
            program_pack::{IsInitialized, Pack},
            rent::Rent,
            system_instruction,
        },
        solana_program_test::*,
        solana_sdk::{
            signature::{Keypair, Signer},
            transaction::Transaction,
        },
        spl_token::{
            instruction::{initialize_account, initialize_mint},
            state::{Account as TokenAccount, Mint},
        },
        std::convert::TryInto,
        std::str::FromStr,
    };

    fn create_mint_transaction(
        payer: &Keypair,
        mint: &Keypair,
        mint_authority: &Keypair,
        recent_blockhash: Hash,
    ) -> Transaction {
        let instructions = [
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                Rent::default().minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                &mint_authority.pubkey(),
                None,
                0,
            )
            .unwrap(),
        ];
        let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
        transaction.partial_sign(&[payer, mint], recent_blockhash);
        transaction
    }

    fn create_token_account_transaction(
        payer: &Keypair,
        mint: &Keypair,
        recent_blockhash: Hash,
        token_account: &Keypair,
        token_account_owner: &Pubkey,
        amount: u64,
    ) -> Transaction {
        let instructions = [
            system_instruction::create_account(
                &payer.pubkey(),
                &token_account.pubkey(),
                Rent::default().minimum_balance(TokenAccount::LEN) + amount,
                TokenAccount::LEN as u64,
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
            processor!(PaymentProcessorInstruction::process),
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
        let result = create_merchant_account().await;
        let program_id = result.0;
        let merchant_keypair = result.1;
        let mut banks_client = result.2;
        let payer = result.3;
        let recent_blockhash = result.4;

        let amount = 2000;
        let order_id = String::from("1337");
        let order_account_size = OrderAccount::MIN_LEN + order_id.chars().count() + 4;

        // first create order account
        let order_keypair = Keypair::new();
        let mut create_order_tx = Transaction::new_with_payer(
            &[system_instruction::create_account(
                &payer.pubkey(),
                &order_keypair.pubkey(),
                Rent::default().minimum_balance(order_account_size),
                order_account_size as u64,
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
        // create and initialize mint
        assert_matches!(
            banks_client
                .process_transaction(create_mint_transaction(
                    &payer,
                    &mint_keypair,
                    &payer,
                    recent_blockhash
                ))
                .await,
            Ok(())
        );
        // create and initialize token
        assert_matches!(
            banks_client
                .process_transaction(create_token_account_transaction(
                    &payer,
                    &mint_keypair,
                    recent_blockhash,
                    &token_keypair,
                    &payer.pubkey(),
                    2000,
                ))
                .await,
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
                amount,
                order_id,
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));

        // test contents of order account
        let order_account = banks_client.get_account(order_keypair.pubkey()).await;
        let order_account = match order_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => value,
            },
            Err(error) => panic!("Problem: {:?}", error),
        };
        let order_data = OrderAccount::unpack(&order_account.data);
        let order_data = match order_data {
            Ok(data) => data,
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(true, order_data.is_initialized());
        assert_eq!(OrderStatus::Paid as u8, order_data.status);
        assert_eq!(
            merchant_keypair.pubkey().to_bytes(),
            order_data.merchant_pubkey
        );
        assert_eq!(
            merchant_keypair.pubkey().to_bytes(),
            order_data.mint_pubkey
        );
        assert_eq!(
            payer.pubkey().to_bytes(),
            order_data.payer_pubkey
        );
        assert_eq!(2000, order_data.expected_amount);
        assert_eq!(0, order_data.paid_amount);
        assert_eq!(0, order_data.fee_amount);
        assert_eq!(String::from("1337"), order_data.order_id);
    }
}
