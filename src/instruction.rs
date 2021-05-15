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
    /// 2. `[]` System program
    /// 3. `[]` The rent sysvar
    /// 4. `[optional]` The sponsor account
    RegisterMerchant {
        /// the seed used when creating the account
        #[allow(dead_code)] // not dead code..
        seed: Option<String>,
        /// the seed used when creating the account
        #[allow(dead_code)] // not dead code..
        data: Option<String>,
    },
    /// Express Checkout - create order and pay for it in one transaction
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person initializing the transaction
    /// 1. `[writable]` The order account.  Owned by this program
    /// 2. `[]` The merchant account.  Owned by this program
    /// 3. `[writable]` The seller token account - this is where the amount paid will go. Owned by this program
    /// 4. `[writable]` The buyer token account
    /// 5. `[writable]` The program owner account (where we will send program owner fee)
    /// 6. `[writable]` The sponsor account (where we will send sponsor fee)
    /// 7. `[]` The token mint account - represents the 'currency' being used
    /// 8. `[]` This program's derived address
    /// 9. `[]` The token program
    /// 10. `[]` The System program
    /// 11. `[]` The clock sysvar
    /// 12. `[]` The rent sysvar
    ExpressCheckout {
        #[allow(dead_code)] // not dead code..
        amount: u64,
        /// the pubkey of the merchant -> this is where the money is to be sent
        /// we are receiving it as data and not an account because during the
        /// express checkout we don't want the UI to have to create this account
        // merchant_token: [u8; 32],
        /// the external order id (as in issued by the merchant)
        #[allow(dead_code)] // not dead code..
        order_id: String,
        // An extra field that can store an encrypted (ot not encrypted) string
        // that the merchant can use to assert if a transaction is authenci
        #[allow(dead_code)] // not dead code..
        secret: String,
    },
    /// Withdraw funds for a particular order
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person initializing the transaction
    /// 1. `[writable]` The order account.  Owned by this program
    /// 2. `[]` The merchant account.  Owned by this program
    /// 3. `[writable]` The order token account (where the money was put during payment)
    /// 4. `[writable]` The merchant token account (where we will withdraw to)
    /// 5. `[]` This program's derived address
    /// 6. `[]` The token program
    /// 7. `[]` The clock sysvar
    Withdraw,
}

/// Creates an 'RegisterMerchant' instruction.
pub fn register_merchant(
    program_id: Pubkey,
    signer: Pubkey,
    merchant: Pubkey,
    seed: Option<String>,
    data: Option<String>,
    sponsor: Option<&Pubkey>,
) -> Instruction {
    let mut account_metas = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(merchant, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    if let Some(sponsor) = sponsor {
        account_metas.push(AccountMeta::new_readonly(*sponsor, false));
    }

    Instruction {
        program_id,
        accounts: account_metas,
        data: PaymentProcessorInstruction::RegisterMerchant { seed, data }
            .try_to_vec()
            .unwrap(),
    }
}

/// Creates an 'ExpressCheckout' instruction.
pub fn express_checkout(
    program_id: Pubkey,
    signer: Pubkey,
    order: Pubkey,
    merchant: Pubkey,
    seller_token: Pubkey,
    buyer_token: Pubkey,
    mint: Pubkey,
    program_owner: Pubkey,
    sponsor: Pubkey,
    pda: Pubkey,
    amount: u64,
    order_id: String,
    secret: String,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(order, false),
            AccountMeta::new_readonly(merchant, false),
            AccountMeta::new(seller_token, false),
            AccountMeta::new(buyer_token, false),
            AccountMeta::new(program_owner, false),
            AccountMeta::new(sponsor, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: PaymentProcessorInstruction::ExpressCheckout {
            amount,
            order_id,
            secret,
        }
        .try_to_vec()
        .unwrap(),
    }
}

/// Creates an 'Withdraw' instruction.
pub fn withdraw(
    program_id: Pubkey,
    signer: Pubkey,
    order: Pubkey,
    merchant: Pubkey,
    order_payment_token: Pubkey,
    merchant_token: Pubkey,
    pda: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(order, false),
            AccountMeta::new_readonly(merchant, false),
            AccountMeta::new(order_payment_token, false),
            AccountMeta::new(merchant_token, false),
            AccountMeta::new_readonly(pda, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: PaymentProcessorInstruction::Withdraw.try_to_vec().unwrap(),
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::engine::constants::{MERCHANT, PDA_SEED, PROGRAM_OWNER},
        crate::instruction::PaymentProcessorInstruction,
        crate::state::{MerchantAccount, OrderAccount, OrderStatus, Serdes},
        crate::utils::{
            get_amounts, get_order_account_pubkey, get_order_account_size, FEE_IN_LAMPORTS,
            SPONSOR_FEE,
        },
        assert_matches::*,
        serde_json::Value,
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
            instruction::{initialize_account, initialize_mint, mint_to},
            state::{Account as TokenAccount, Mint},
        },
        std::str::FromStr,
    };

    type MerchantResult = (Pubkey, Pubkey, BanksClient, Keypair, Hash);

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
                Rent::default().minimum_balance(TokenAccount::LEN),
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
            mint_to(
                &spl_token::id(),
                &mint.pubkey(),
                &token_account.pubkey(),
                token_account_owner,
                &[&payer.pubkey()],
                amount,
            )
            .unwrap(),
        ];
        let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
        transaction.partial_sign(&[payer, token_account], recent_blockhash);
        transaction
    }

    async fn create_merchant_account(
        seed: Option<String>,
        sponsor: Option<&Pubkey>,
        data: Option<String>,
    ) -> MerchantResult {
        let program_id = Pubkey::from_str(&"mosh111111111111111111111111111111111111111").unwrap();

        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "sol_payment_processor",
            program_id,
            processor!(PaymentProcessorInstruction::process),
        )
        .start()
        .await;

        let real_seed = match &seed {
            None => MERCHANT,
            Some(value) => &value,
        };

        // first we create a public key for the merchant account
        let merchant_acc_pubkey =
            Pubkey::create_with_seed(&payer.pubkey(), real_seed, &program_id).unwrap();

        // then call register merchant ix
        let mut transaction = Transaction::new_with_payer(
            &[register_merchant(
                program_id,
                payer.pubkey(),
                merchant_acc_pubkey,
                Some(real_seed.to_string()),
                data,
                sponsor,
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
        return (
            program_id,
            merchant_acc_pubkey,
            banks_client,
            payer,
            recent_blockhash,
        );
    }

    async fn create_order_account(
        order_id: &String,
        amount: u64,
        secret: &String,
        program_id: &Pubkey,
        merchant: &Pubkey,
        buyer_token: &Pubkey,
        mint: &Pubkey,
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: Hash,
    ) -> (Pubkey, Pubkey) {
        let order_acc = get_order_account_pubkey(&order_id, &payer.pubkey(), program_id);
        let (pda, _bump_seed) = Pubkey::find_program_address(&[PDA_SEED], &program_id);

        let (seller_token, _bump_seed) = Pubkey::find_program_address(
            &[
                &order_acc.to_bytes(),
                &spl_token::id().to_bytes(),
                &mint.to_bytes(),
            ],
            program_id,
        );

        let merchant_account = banks_client.get_account(*merchant).await;
        let merchant_data = match merchant_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => match MerchantAccount::unpack(&value.data) {
                    Ok(data) => data,
                    Err(error) => panic!("Problem: {:?}", error),
                },
            },
            Err(error) => panic!("Problem: {:?}", error),
        };

        // call express checkout ix
        let mut transaction = Transaction::new_with_payer(
            &[express_checkout(
                *program_id,
                payer.pubkey(),
                order_acc,
                *merchant,
                seller_token,
                *buyer_token,
                *mint,
                Pubkey::from_str(PROGRAM_OWNER).unwrap(),
                Pubkey::new_from_array(merchant_data.sponsor),
                pda,
                amount,
                (&order_id).to_string(),
                (&secret).to_string(),
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));

        (order_acc, seller_token)
    }

    async fn create_order(
        amount: u64,
        order_id: &String,
        secret: &String,
        merchant_result: &mut MerchantResult,
    ) -> (Pubkey, Pubkey, Keypair) {
        let program_id = merchant_result.0;
        let merchant_account_pubkey = merchant_result.1;
        let mut banks_client = &mut merchant_result.2;
        let payer = &merchant_result.3;
        let recent_blockhash = merchant_result.4;

        // next create token account for test
        let mint_keypair = Keypair::new();
        let buyer_token_keypair = Keypair::new();

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
        // create and initialize buyer token account
        assert_matches!(
            banks_client
                .process_transaction(create_token_account_transaction(
                    &payer,
                    &mint_keypair,
                    recent_blockhash,
                    &buyer_token_keypair,
                    &payer.pubkey(),
                    amount + 2000000,
                ))
                .await,
            Ok(())
        );

        let (order_acc, seller_account) = create_order_account(
            &order_id,
            amount,
            &secret,
            &program_id,
            &merchant_account_pubkey,
            &buyer_token_keypair.pubkey(),
            &mint_keypair.pubkey(),
            &mut banks_client,
            &payer,
            recent_blockhash,
        )
        .await;

        (order_acc, seller_account, mint_keypair)
    }

    async fn run_merchant_tests(result: MerchantResult) -> MerchantAccount {
        let program_id = result.0;
        let merchant = result.1;
        let mut banks_client = result.2;
        let payer = result.3;
        // test contents of merchant account
        let merchant_account = banks_client.get_account(merchant).await;
        let merchant_account = match merchant_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => value,
            },
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(merchant_account.owner, program_id);
        let merchant_data = MerchantAccount::unpack(&merchant_account.data);
        let merchant_data = match merchant_data {
            Ok(data) => data,
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(true, merchant_data.is_initialized());
        assert_eq!(payer.pubkey(), Pubkey::new_from_array(merchant_data.owner));

        merchant_data
    }

    #[tokio::test]
    async fn test_register_merchant() {
        let result = create_merchant_account(Option::None, Option::None, Option::None).await;
        let merchant_data = run_merchant_tests(result).await;
        assert_eq!(String::from("{}"), merchant_data.data);
    }

    #[tokio::test]
    async fn test_register_merchant_with_seed() {
        let result =
            create_merchant_account(Some(String::from("mosh")), Option::None, Option::None).await;
        let merchant = result.1;
        let payer = result.3;
        let program_id = result.0;
        assert_eq!(
            merchant,
            Pubkey::create_with_seed(&payer.pubkey(), "mosh", &program_id).unwrap()
        );
    }

    #[tokio::test]
    async fn test_register_merchant_with_all_stuff() {
        let seed = String::from("mosh");
        let sponsor_pk = Pubkey::new_unique();
        let data = String::from(
            r#"{"code":200,"success":true,"payload":{"features":["awesome","easyAPI","lowLearningCurve"]}}"#,
        );
        let datas = data.clone();
        let result = create_merchant_account(Some(seed), Some(&sponsor_pk), Some(data)).await;
        let merchant_data = run_merchant_tests(result).await;
        assert_eq!(datas, merchant_data.data);
        assert_eq!(sponsor_pk, Pubkey::new_from_array(merchant_data.sponsor));
        // just for sanity verify that you can get some of the JSON values
        let json_value: Value = serde_json::from_str(&merchant_data.data).unwrap();
        assert_eq!(200, json_value["code"]);
        assert_eq!(true, json_value["success"]);
    }

    async fn run_checkout_tests(
        amount: u64,
        order_id: String,
        secret: String,
        merchant_result: MerchantResult,
        order_acc_pubkey: Pubkey,
        seller_account_pubkey: Pubkey,
        mint_keypair: Keypair,
    ) {
        let program_id = merchant_result.0;
        let merchant_account_pubkey = merchant_result.1;
        let mut banks_client = merchant_result.2;
        let payer = merchant_result.3;

        // test contents of order account
        let order_account = banks_client.get_account(order_acc_pubkey).await;
        let order_account = match order_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => value,
            },
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(order_account.owner, program_id);
        assert_eq!(
            order_account.lamports,
            Rent::default().minimum_balance(get_order_account_size(&order_id, &secret))
        );
        let order_data = OrderAccount::unpack(&order_account.data);
        let order_data = match order_data {
            Ok(data) => data,
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(true, order_data.is_initialized());
        assert_eq!(OrderStatus::Paid as u8, order_data.status);
        assert_eq!(merchant_account_pubkey.to_bytes(), order_data.merchant);
        assert_eq!(mint_keypair.pubkey().to_bytes(), order_data.mint);
        assert_eq!(seller_account_pubkey.to_bytes(), order_data.token);
        assert_eq!(merchant_account_pubkey.to_bytes(), order_data.merchant);
        assert_eq!(payer.pubkey().to_bytes(), order_data.payer);
        assert_eq!(amount, order_data.expected_amount);
        assert_eq!(amount, order_data.paid_amount);
        assert_eq!(order_id, order_data.order_id);
        assert_eq!(secret, order_data.secret);

        // test contents of seller token account
        let seller_token_account = banks_client.get_account(seller_account_pubkey).await;
        let seller_token_account = match seller_token_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => value,
            },
            Err(error) => panic!("Problem: {:?}", error),
        };
        let seller_account_data = spl_token::state::Account::unpack(&seller_token_account.data);
        let seller_account_data = match seller_account_data {
            Ok(data) => data,
            Err(error) => panic!("Problem: {:?}", error),
        };
        let (pda, _bump_seed) = Pubkey::find_program_address(&[PDA_SEED], &program_id);
        assert_eq!(amount, seller_account_data.amount);
        assert_eq!(pda, seller_account_data.owner);
        assert_eq!(mint_keypair.pubkey(), seller_account_data.mint);

        // test that sponsor was saved okay
        let merchant_account = banks_client.get_account(merchant_account_pubkey).await;
        let merchant_data = match merchant_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => match MerchantAccount::unpack(&value.data) {
                    Ok(data) => data,
                    Err(error) => panic!("Problem: {:?}", error),
                },
            },
            Err(error) => panic!("Problem: {:?}", error),
        };

        let program_owner_key = Pubkey::from_str(PROGRAM_OWNER).unwrap();
        let sponsor = Pubkey::new_from_array(merchant_data.sponsor);

        let program_owner_account = banks_client.get_account(program_owner_key).await;
        let program_owner_account = match program_owner_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => value,
            },
            Err(error) => panic!("Problem: {:?}", error),
        };

        if sponsor == program_owner_key {
            // test contents of program owner account
            assert_eq!(FEE_IN_LAMPORTS, program_owner_account.lamports);
        } else {
            // test contents of program owner account and sponsor account
            let (program_owner_fee, sponsor_fee) = get_amounts(FEE_IN_LAMPORTS, SPONSOR_FEE);
            let sponsor_account = banks_client.get_account(sponsor).await;
            let sponsor_account = match sponsor_account {
                Ok(data) => match data {
                    None => panic!("Oo"),
                    Some(value) => value,
                },
                Err(error) => panic!("Problem: {:?}", error),
            };
            assert_eq!(program_owner_fee, program_owner_account.lamports);
            assert_eq!(sponsor_fee, sponsor_account.lamports);
        }
    }

    #[tokio::test]
    async fn test_express_checkout() {
        let amount: u64 = 2000000000;
        let order_id = String::from("1337");
        let secret = String::from("hunter2");
        let mut merchant_result =
            create_merchant_account(Option::None, Option::None, Option::None).await;
        let (order_acc_pubkey, seller_account_pubkey, mint_keypair) =
            create_order(amount, &order_id, &secret, &mut merchant_result).await;

        run_checkout_tests(
            amount,
            order_id,
            secret,
            merchant_result,
            order_acc_pubkey,
            seller_account_pubkey,
            mint_keypair,
        )
        .await;
    }

    #[tokio::test]
    async fn test_express_checkout_with_sponsor() {
        let sponsor_pk = Pubkey::new_unique();
        let amount: u64 = 2000000000;
        let order_id = String::from("123-SQT-MX");
        let secret = String::from("supersecret");
        let mut merchant_result =
            create_merchant_account(Option::None, Some(&sponsor_pk), Option::None).await;
        let (order_acc_pubkey, seller_account_pubkey, mint_keypair) =
            create_order(amount, &order_id, &secret, &mut merchant_result).await;

        run_checkout_tests(
            amount,
            order_id,
            secret,
            merchant_result,
            order_acc_pubkey,
            seller_account_pubkey,
            mint_keypair,
        )
        .await;
    }

    #[tokio::test]
    async fn test_withdraw() {
        let mut merchant_result =
            create_merchant_account(Option::None, Option::None, Option::None).await;
        let merchant_token_keypair = Keypair::new();
        let amount: u64 = 1234567890;
        let order_id = String::from("PD17CUSZ75");
        let secret = String::from("i love oov");
        let (order_acc_pubkey, _seller_account_pubkey, mint_keypair) =
            create_order(amount, &order_id, &secret, &mut merchant_result).await;
        let program_id = merchant_result.0;
        let merchant_account_pubkey = merchant_result.1;
        let mut banks_client = merchant_result.2;
        let payer = merchant_result.3;
        let recent_blockhash = merchant_result.4;
        let (pda, _bump_seed) = Pubkey::find_program_address(&[PDA_SEED], &program_id);

        // create and initialize merchant token account
        assert_matches!(
            banks_client
                .process_transaction(create_token_account_transaction(
                    &payer,
                    &mint_keypair,
                    recent_blockhash,
                    &merchant_token_keypair,
                    &payer.pubkey(),
                    0,
                ))
                .await,
            Ok(())
        );
        let (order_payment_token_acc_pubkey, _bump_seed) = Pubkey::find_program_address(
            &[
                &order_acc_pubkey.to_bytes(),
                &spl_token::id().to_bytes(),
                &mint_keypair.pubkey().to_bytes(),
            ],
            &program_id,
        );

        // call withdraw ix
        let mut transaction = Transaction::new_with_payer(
            &[withdraw(
                program_id,
                payer.pubkey(),
                order_acc_pubkey,
                merchant_account_pubkey,
                order_payment_token_acc_pubkey,
                merchant_token_keypair.pubkey(),
                pda,
            )],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));

        // test contents of order account
        let order_account = banks_client.get_account(order_acc_pubkey).await;
        let order_data = match order_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => match OrderAccount::unpack(&value.data) {
                    Ok(data) => data,
                    Err(error) => panic!("Problem: {:?}", error),
                },
            },
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(OrderStatus::Withdrawn as u8, order_data.status);
        assert_eq!(amount, order_data.expected_amount);
        assert_eq!(amount, order_data.paid_amount);
        assert_eq!(order_id, order_data.order_id);
        assert_eq!(secret, order_data.secret);

        // test contents of merchant token account
        let merchant_token_account = banks_client
            .get_account(merchant_token_keypair.pubkey())
            .await;
        let merchant_account_data = match merchant_token_account {
            Ok(data) => match data {
                None => panic!("Oo"),
                Some(value) => match spl_token::state::Account::unpack(&value.data) {
                    Ok(data) => data,
                    Err(error) => panic!("Problem: {:?}", error),
                },
            },
            Err(error) => panic!("Problem: {:?}", error),
        };
        assert_eq!(order_data.paid_amount, merchant_account_data.amount);
    }
}
