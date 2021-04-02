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
    /// Seller And Buyer Mints Not The Same
    #[error("Error: Seller And Buyer Mints Not The Same")]
    MintNotEqual,
    /// The Amount Is Already Withdrawn
    #[error("Error: The Amount Is Already Withdrawn")]
    AlreadyWithdrawn,
    /// The Provided Merchant Is Wrong
    #[error("Error: The Provided Merchant Is Wrong")]
    WrongMerchant,
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
