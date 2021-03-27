use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum PaymentProcessorError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    /// Seller And Buyer Mints Not The Same
    #[error("Seller And Buyer Mints Not The Same")]
    MintNotEqual,
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
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            PaymentProcessorError::InvalidInstruction => msg!("Error: Invalid Instruction"),
            PaymentProcessorError::MintNotEqual => {
                msg!("Error: Seller And Buyer Mints Not The Same")
            }
        }
    }
}
