//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq, FromPrimitive)]
pub enum PaymentProcessorError {
    /// Invalid instruction
    #[error("Error: Invalid Instruction")]
    InvalidInstruction,
    /// Invalid Subscription Data
    #[error("Error: Invalid Subscription Data")]
    InvalidSubscriptionData,
    /// Invalid Subscription Package
    #[error("Error: Invalid Subscription Package")]
    InvalidSubscriptionPackage,
    /// Seller And Buyer Mints Not The Same
    #[error("Error: Seller And Buyer Mints Not The Same")]
    MintNotEqual,
    /// The Amount Is Already Withdrawn
    #[error("Error: The Amount Is Already Withdrawn")]
    AlreadyWithdrawn,
    /// The Order Account Is Invalid
    #[error("Error: The Order Account Is Invalid")]
    InvalidOrder,
    /// The Payment Has Not Been Received In Full
    #[error("Error: The Payment Has Not Been Received In Full")]
    NotFullyPaid,
    /// The Payment Has Not Yet Been Made
    #[error("Error: The Payment Has Not Yet Been Made")]
    NotPaid,
    /// The Provided Merchant Is Wrong
    #[error("Error: The Provided Merchant Is Wrong")]
    WrongMerchant,
    /// The Payer Is Wrong
    #[error("Error: The Payer Is Wrong")]
    WrongPayer,
    /// The Provided Program Owner Is Wrong
    #[error("Error: The Provided Program Owner Is Wrong")]
    WrongProgramOwner,
    /// The Provided Sponsor Is Wrong
    #[error("Error: The Provided Sponsor Is Wrong")]
    WrongSponsor,
}

impl From<PaymentProcessorError> for ProgramError {
    fn from(e: PaymentProcessorError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for PaymentProcessorError {
    fn type_of() -> &'static str {
        "Solana Payment Processor Error"
    }
}

impl PrintProgramError for PaymentProcessorError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}
