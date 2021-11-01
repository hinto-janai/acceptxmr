# `AcceptXMR`: An Embedded Monero Payment Processor

This library aims to provide a simple, reliable, and efficient means to track monero payments in
your application.

To track payments, the `PaymentGateway` generates subaddresses using your private view key and
public spend key. It then watches for monero sent to that subaddress using a monero daemon of your
choosing, your private view key and your public spend key.

Use this library at your own risk, it is young and unproven.

## Key Features
* View pair only, no hot wallet.
* Subaddress based. 
* Pending invoices stored persistently, enabling recovery from power loss. 
* Number of confirmations is configurable per-invoice.
* Ignores transactions with non-zero timelocks.

## Security

`AcceptXMR` is non-custodial, and does not require a hot wallet. However, it does require your
private view key and public spend key for scanning outputs. If keeping these private is important
to you, please take appropriate precautions to secure the platform you run your application on.

Also note that anonymity networks like TOR are not currently supported for RPC calls. This
means that your network traffic will reveal that you are interacting with the monero network.

## Reliability

This library strives for reliability, but that attempt may not be successful. `AcceptXMR` is
young and unproven, and relies on several crates which are undergoing rapid changes themselves
(for example, the database used ([`Sled`](https://docs.rs/sled)) is still in beta).

That said, this payment gateway should survive unexpected power loss thanks to pending invoices
being flushed to disk each time new blocks/transactions are scanned. A best effort is made to keep
the scanning thread free any of potential panics, and RPC calls in the scanning thread are logged on
failure and repeated next scan. In the event that an error does occur, the liberal use of logging
within this library will hopefully facilitate a speedy diagnosis an correction.

Use this library at your own risk.

## Performance

For maximum performance, please host your own monero daemon the same local network. Network and
daemon slowness are primary cause of high invoice update latency in the majority of use cases.

To reduce the average latency before receiving invoice updates, you may also consider lowering
the `PaymentGateway`'s `scan_interval` below the default of 1 second:
```rust
use acceptxmr::PaymentGateway;
use std::time::Duration;

let private_view_key = "ad2093a5705b9f33e6f0f0c1bc1f5f639c756cdfc168c8f2ac6127ccbdab3a03";
let public_spend_key = "7388a06bd5455b793a82b90ae801efb9cc0da7156df8af1d5800e4315cc627b4";

let payment_gateway = PaymentGateway::builder(private_view_key, public_spend_key)
    .scan_interval(Duration::from_millis(100)) // Scan for invoice updates every 100 ms.
    .build();
```

Please note that `scan_interval` is the minimum time between scanning for updates. If your
daemon's response time is already greater than your `scan_interval`, or if your CPU is unable to
scan new transactions fast enough, reducing your `scan_interval` will do nothing.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
