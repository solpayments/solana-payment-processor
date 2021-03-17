use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum PaymentProcessorError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    /// Not Rent Exempt
    #[error("Not Rent Exempt")]
    NotRentExempt,
}

impl From<PaymentProcessorError> for ProgramError {
    fn from(e: PaymentProcessorError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
