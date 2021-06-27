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

All the instructions supported by the Sol Payments program are documented [here](src/instruction.rs).

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
