# Sol Payments

SolPayments is a smart contract program build for the [Solana blockchain](https://solana.com/) that allows merchants to receive crypto-currency payments at their online shops.

## Benefits of using SolPayments

- **Low fees** - SolPayments charges merchants just 0.3% of the transaction value.  Additionally, the fees charged to buyers for sending the crypto-currency payments are almost free (just a few cents)
- **Fast** - payments made through SolPayments are completed in a few seconds
- **Non-custodial** - SolPayments never takes custody of payments made to any merchants that use it.  You are always in full control of your money.

## How does it work?

1. It starts with merchant registration where the merchant registers with SolPayments.  At this stage, the merchant can optionally identify a sponsor (the person who referred them to SolPayments or helped them set up).
2. For accepting payments, the merchant would provide a user interface (UI) which creates a unique order id and generates a "pay now" or similar button.  When this button is pressed, the following happens:
    - an `ExpressCheckout` instruction is sent to the program
    - an order account is created on chain that stores the order details
    - the payment amount is transferred to a token account that is controlled by the program (to the extent that no one other than the merchant can access this amount)
3. Finally, the merchant (or anyone at all) can issue in instruction to withdraw the amount.  This results in the payment being transferred to the merchant, and the payment processing fees being sent to the program owner and/or the sponsor.

### Subscriptions

The above basic functionality can be though of as a basic primitive that allows for the support of all kinds of e-commerce payments.  This is actually how suscriptions work.

- Register a special merchant account that specifies the available subscription packages and their costs
- Make a payment using `ExpressCheckout` that specifies the merchant account
- Call the `Subscribe` instruction and specify the merchant and order accounts.  The program will create a subscription account that specifies the subscription package and the date of expiry.

## Program API

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
RegisterMerchant {
    /// the seed used when creating the account
    seed: Option<String>,
    /// the amount (in SOL lamports) that will be charged as a fee
    fee: Option<u64>,
    /// the seed used when creating the account
    data: Option<String>,
}
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
/// 5. `[writable]` The program owner account (where we will send program owner fee)
/// 6. `[writable]` The sponsor account (where we will send sponsor fee)
/// 7. `[]` The token mint account - represents the 'currency' being used
/// 8. `[]` This program's derived address
/// 9. `[]` The token program
/// 10. `[]` The System program
/// 11. `[]` The clock sysvar
/// 12. `[]` The rent sysvar
ExpressCheckout {
    amount: u64,
    /// the pubkey of the merchant -> this is where the money is to be sent
    /// we are receiving it as data and not an account because during the
    /// express checkout we don't want the UI to have to create this account
    // merchant_token: [u8; 32],
    /// the external order id (as in issued by the merchant)
    order_id: String,
    // An extra field that can store an encrypted (ot not encrypted) string
    // that the merchant can use to assert if a transaction is authenci
    secret: String,
    /// arbitrary merchant data (maybe as a JSON string)
    data: Option<String>,
}
```

#### Expected inputs

- amount: the amount being paid in this transaction
- order_id: an order ID that the merchant can use to track what this payment is for
- secret: a secret value (can be encrypted) that the merchant's ecommerce software can use to verify that this payment and order are valid

### 3. Start Subscription

In which a user pays a subscription and gets back a subscription account that shows which package they are subscribed to and when it expires.

```rust
/// Initialize a subscription
///
/// Accounts expected:
///
/// 0. `[signer]` The account of the person initializing the transaction
/// 1. `[writable]` The subscription account.  Owned by this program
/// 2. `[]` The merchant account.  Owned by this program
/// 3. `[]` The order account.  Owned by this program
/// 4. `[]` The System program
/// 5. `[]` The clock sysvar
/// 6. `[]` The rent sysvar
Subscribe {
    /// the subscription package name
    name: String,
    /// arbitrary merchant data (maybe as a JSON string)
    data: Option<String>,
}
```

This instruction is meant to be ran just after creating and paying for an order using the `ExpressCheckout` instruction.

### 4. Withdraw Funds

Whenever ready anyone can execute a `Withdraw` instruction which will result in the merchant receiving th payment in their own account.

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
/// 5. `[]` This program's derived address
/// 6. `[]` The token program
/// 7. `[]` The clock sysvar
Withdraw
```

## Contributing

### Environment Setup

1. Install Rust from [https://rustup.rs/](https://rustup.rs/)
2. Install Solana v1.5.0 or later from [https://docs.solana.com/cli/install-solana-cli-tools#use-solanas-install-tool](https://docs.solana.com/cli/install-solana-cli-tools#use-solanas-install-tool)

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
