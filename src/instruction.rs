use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

use crate::error::PaymentProcessorError::InvalidInstruction;

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, BorshSchema, PartialEq)]
pub enum PaymentProcessorInstruction {
    /// Register for a merchant account.
    ///
    /// Accounts expected:
    ///
    /// 0. `[signer]` The account of the person initializing the loan
    /// 1. `[writable]` The merchant account.  Owned by this program
    /// 2. `[]` The rent sysvar
    RegisterMerchant,
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
        .pack_into_vec(),
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        assert_matches::*,
        solana_program::{
            hash::Hash,
            rent::Rent,
            system_instruction,
        },
        solana_program_test::*,
        solana_sdk::{
            account::Account,
            // clock::Epoch,
            signature::{Keypair, Signer},
            transaction::Transaction,
            transport::TransportError,
        },
        spl_token,
        std::str::FromStr,
        // std::mem,
        crate::processor::{ process_instruction },
    };

    // fn create_account<'a>(
    //     is_signer: bool,
    //     is_writable: bool,
    //     pk: &'a Pubkey,
    //     owner: &'a Pubkey,
    //     lamports: &'a mut u64,
    //     data: &'a mut [u8],
    // ) -> AccountInfo<'a> {
    //     AccountInfo::new(
    //         &pk,
    //         is_signer,
    //         is_writable,
    //         lamports,
    //         data,
    //         &owner,
    //         false,
    //         Epoch::default(),
    //     )
    // }

    pub async fn create_account(
        banks_client: &mut BanksClient,
        payer: &Keypair,
        recent_blockhash: &Hash,
        account: &Keypair,
        pool_mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<(), TransportError> {
        let rent = banks_client.get_rent().await.unwrap();
        let account_rent = rent.minimum_balance(165);

        let mut transaction = Transaction::new_with_payer(
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &account.pubkey(),
                    account_rent,
                    165 as u64,
                    &spl_token::id(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[payer, account], *recent_blockhash);
        banks_client.process_transaction(transaction).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_register_merchant() {
        let program_id = Pubkey::from_str(&"mosh111111111111111111111111111111111111111").unwrap();
        let account_key = Pubkey::new_unique();
        let merchant_key = Pubkey::new_unique();

        // let mut lamports = 2000000;
        // let mut data = vec![0; 0];

        // let signer_acc = create_account(
        //     true,
        //     false,
        //     &account_key,
        //     &spl_token::ID,
        //     &mut lamports,
        //     &mut data,
        // );

        // let merchant_acc = create_account(
        //     false,
        //     true,
        //     &merchant_key,
        //     &program_id,
        //     &mut Rent::default().minimum_balance(33),
        //     &mut data,
        // );

        // let signer_account = Account::new(2000000, 33, &account_key);
        // let merchant_acc = Account::new(
        //     Rent::default().minimum_balance(33),
        //     33,
        //     &program_id,
        // );

        let instruction = register_merchant(program_id, account_key, merchant_key);

        let mut test = ProgramTest::new(
            "bpf_program_template",
            program_id,
            processor!(process_instruction),
        );

        test.add_account(account_key, Account {
            lamports: 2000000,
            // data: vec![0_u8; mem::size_of::<u32>()],
            owner: spl_token::ID,
            ..Account::default()
        });
        test.add_account(merchant_key, Account {
            lamports: Rent::default().minimum_balance(33),
            // data: vec![0_u8; mem::size_of::<u32>()],
            owner: program_id,
            ..Account::default()
        });

        let (mut banks_client, payer, recent_blockhash) = test
        .start()
        .await;

        let mut transaction = Transaction::new_with_payer(
            &[instruction],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer, &account_key], recent_blockhash);

        assert_matches!(banks_client.process_transaction(transaction).await, Ok(()));
    }
}