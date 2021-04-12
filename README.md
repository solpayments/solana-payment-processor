# Sol Payments

SolPayments is a smart contract program build for the [Solana blockchain](https://solana.com/) that allows merchants to receive crypto-currency payments at their online shops.

## Benefits of using SolPayments

- **Low fees** - SolPayments charges merchants just 0.3% of the transaction value.  Additionally, the fees charged to buyers for sending the crypto-currency payments are almost free (just a few cents)
- **Fast** - payments made through SolPayments are completed in a few seconds
- **Non-custodial** - SolPayments never takes custody of payments made to any merchants that use it.  You are always in full control of your money.

## SolPayments Program API

The SolPayments program has three general parts:

### 1. Merchant Registration

Each merchant needs to register as a merchant with SolPayments by issuing a `RegisterMerchant` instruction.

```rust
/// Register for a merchant account.
///
/// Accounts expected:
///
/// 0. `[signer]` The account of the person initializing the merchant account
/// 1. `[writable]` The merchant account.  Owned by this program
/// 2. `[]` System program
/// 3. `[]` The rent sysvar
/// 4. `[optional]` The sponsor account
RegisterMerchant
```

Under the hood, this results in an on-chain "Merchant Account" being created and controlled by the SolPayments program.

### 2. Express Checkout

Next, a person who wishes to make a payment to the merchant can issue an `ExpressCheckout` instruction:

```rust
/// Express Checkout - create order and pay for it in one transaction
///
/// Accounts expected:
///
/// 0. `[signer]` The account of the person initializing the transaction
/// 1. `[writable]` The order account.  Owned by this program
/// 2. `[]` The merchant account.  Owned by this program
/// 3. `[writable]` The seller token account - this is where the amount paid will go. Owned by this program
/// 4. `[writable]` The buyer token account
/// 5. `[]` The token mint account - represents the 'currency' being used
/// 6. `[]` This program's derived address
/// 7. `[]` The token program
/// 8. `[]` The System program
/// 9. `[]` The clock sysvar
/// 10. `[]` The rent sysvar
ExpressCheckout {
    amount: u64,
    /// the pubkey of the merchant -> this is where the money is to be sent
    /// we are receiving it as data and not an account because during the
    /// express checkout we don't want the UI to have to create this account
    // merchant_token_pubkey: [u8; 32],
    /// the external order id (as in issued by the merchant)
    order_id: String,
    // An extra field that can store an encrypted (ot not encrypted) string
    // that the merchant can use to assert if a transaction is authenci
    secret: String,
}
```

#### Expected inputs

- amount: the amount being paid in this transaction
- order_id: an order ID that the merchant can use to track what this payment is for
- secret: a secret value (can be encrypted) that the merchant's ecommerce software can use to verify that this payment and order are valid

### 3. Withdraw Funds

Finally, whenever ready anyone can execute a `Withdraw` instruction which will result in the merchant receiving th payment in their own account.

```rust
/// Withdraw funds for a particular order
///
/// Accounts expected:
///
/// 0. `[signer]` The account of the person initializing the transaction
/// 1. `[writable]` The order account.  Owned by this program
/// 2. `[]` The merchant account.  Owned by this program
/// 3. `[writable]` The order token account (where the money was put during payment)
/// 4. `[writable]` The merchant token account (where we will withdraw to)
/// 5. `[writable]` The program owner token account (where we will send program owner fee)
/// 6. `[writable]` The sponsor token account (where we will send sponsor fee)
/// 7. `[]` The sol-payment-processor program derived address
/// 8. `[]` The token program
/// 9. `[]` The clock sysvar
Withdraw,
```

## Contributing

### Environment Setup

1. Install Rust from https://rustup.rs/
2. Install Solana v1.5.0 or later from https://docs.solana.com/cli/install-solana-cli-tools#use-solanas-install-tool

### Build and test for program compiled natively

```sh
$ cargo build
$ cargo test
```

### Build and test the program compiled for BPF

```sh
$ cargo build-bpf
$ cargo test-bpf
```
