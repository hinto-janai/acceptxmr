mod common;

use std::time::Duration;

use tokio::runtime::Runtime;

use acceptxmr::{PaymentGatewayBuilder, SubIndex};

use crate::common::MockDaemon;

#[test]
fn run_payment_gateway() {
    // Setup.
    common::init_logger();
    let temp_dir = common::new_temp_dir();
    let mock_daemon = MockDaemon::new_mock_daemon();
    let rt = Runtime::new().expect("failed to create tokio runtime");

    // Create payment gateway pointing at temp directory and mock daemon.
    let payment_gateway =
        PaymentGatewayBuilder::new(common::PRIVATE_VIEW_KEY, common::PUBLIC_SPEND_KEY)
            .db_path(
                temp_dir
                    .path()
                    .to_str()
                    .expect("failed to get temporary directory path"),
            )
            .daemon_url(&mock_daemon.url(""))
            .build();

    // Run it.
    rt.block_on(async {
        payment_gateway
            .run()
            .await
            .expect("failed to run payment gateway");
    })
}

#[test]
fn new_payment() {
    // Setup.
    common::init_logger();
    let temp_dir = common::new_temp_dir();
    let mock_daemon = MockDaemon::new_mock_daemon();
    let rt = Runtime::new().expect("failed to create tokio runtime");

    // Create payment gateway pointing at temp directory and mock daemon.
    let payment_gateway =
        PaymentGatewayBuilder::new(common::PRIVATE_VIEW_KEY, common::PUBLIC_SPEND_KEY)
            .db_path(
                temp_dir
                    .path()
                    .to_str()
                    .expect("failed to get temporary directory path"),
            )
            // Faster scan rate so the update is received sooner.
            .scan_interval(Duration::from_millis(100))
            .daemon_url(&mock_daemon.url(""))
            .build();

    // Run it.
    rt.block_on(async {
        payment_gateway
            .run()
            .await
            .expect("failed to run payment gateway");

        // Add the payment.
        let mut subscriber = payment_gateway
            .new_payment(1, 5, 10)
            .await
            .expect("failed to add new payment to payment gateway for tracking");

        // Get initial update.
        let update = subscriber
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        // Check that it is as expected.
        assert_eq!(update.amount_requested(), 1);
        assert_eq!(update.amount_paid(), 0);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 10);
        assert_eq!(update.started_at(), update.current_height());
        assert_eq!(update.confirmations_required(), 5);
        assert_eq!(update.confirmations(), None);
    })
}

#[test]
fn track_parallel_payments() {
    // Setup.
    common::init_logger();
    let temp_dir = common::new_temp_dir();
    let mock_daemon = MockDaemon::new_mock_daemon();
    let rt = Runtime::new().expect("failed to create tokio runtime");

    // Create payment gateway pointing at temp directory and mock daemon.
    let payment_gateway =
        PaymentGatewayBuilder::new(common::PRIVATE_VIEW_KEY, common::PUBLIC_SPEND_KEY)
            .db_path(
                temp_dir
                    .path()
                    .to_str()
                    .expect("failed to get temporary directory path"),
            )
            // Faster scan rate so the update is received sooner.
            .scan_interval(Duration::from_millis(100))
            .daemon_url(&mock_daemon.url(""))
            .seed(1)
            .build();

    // Run it.
    rt.block_on(async {
        payment_gateway
            .run()
            .await
            .expect("failed to run payment gateway");

        // Add the payment.
        let mut subscriber_1 = payment_gateway
            .new_payment(70000000, 2, 7)
            .await
            .expect("failed to add new payment to payment gateway for tracking");

        // Get initial update.
        let update = subscriber_1
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        // Check that it is as expected.
        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 97));
        assert_eq!(update.amount_paid(), 0);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.started_at(), update.current_height());
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), None);

        // Add the payment.
        let mut subscriber_2 = payment_gateway
            .new_payment(70000000, 2, 7)
            .await
            .expect("failed to add new payment to payment gateway for tracking");

        // Get initial update.
        let update = subscriber_2
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        // Check that it is as expected.
        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 138));
        assert_eq!(update.amount_paid(), 0);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.started_at(), update.current_height());
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), None);

        // Add double transfer to txpool.
        let txpool_hashes_mock =
            mock_daemon.mock_txpool_hashes("tests/rpc_resources/txpool_hashes_with_payment.json");
        // Mock for these transactions themselves is unnecessary, because they are all in block
        // 2477657.

        // Get update.
        let update = subscriber_1
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        // Check that it is as expected.
        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 97));
        assert_eq!(update.amount_paid(), 37419570);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.started_at(), update.current_height());
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), None);

        // Get update.
        let update = subscriber_2
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        // Check that it is as expected.
        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 138));
        assert_eq!(update.amount_paid(), 37419570);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.started_at(), update.current_height());
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), None);

        // Check that the mock server did in fact receive the requests.
        assert!(txpool_hashes_mock.hits() > 0);

        // Move forward a few blocks.
        mock_daemon.mock_txpool_hashes("tests/rpc_resources/txpool_hashes.json");
        for height in 2477657..2477662 {
            let height_mock = mock_daemon.mock_daemon_height(height);

            let update = subscriber_1
                .recv_timeout(Duration::from_millis(2000))
                .expect("failed to retrieve payment update");

            assert_eq!(update.amount_requested(), 70000000);
            assert_eq!(update.index(), SubIndex::new(1, 97));
            assert_eq!(update.amount_paid(), 37419570);
            assert!(!update.is_expired());
            assert!(!update.is_confirmed());
            assert_eq!(update.expiration_at() - update.started_at(), 7);
            assert_eq!(update.current_height(), height);
            assert_eq!(update.confirmations_required(), 2);
            assert_eq!(update.confirmations(), None);

            let update = subscriber_2
                .recv_timeout(Duration::from_millis(2000))
                .expect("failed to retrieve payment update");

            assert_eq!(update.amount_requested(), 70000000);
            assert_eq!(update.index(), SubIndex::new(1, 138));
            assert_eq!(update.amount_paid(), 37419570);
            assert!(!update.is_expired());
            assert!(!update.is_confirmed());
            assert_eq!(update.expiration_at() - update.started_at(), 7);
            assert_eq!(update.current_height(), height);
            assert_eq!(update.confirmations_required(), 2);
            assert_eq!(update.confirmations(), None);

            assert!(height_mock.hits() > 0);
        }

        // Put second payment in txpool.
        let txpool_hashes_mock =
            mock_daemon.mock_txpool_hashes("tests/rpc_resources/txpool_hashes_with_payment_2.json");
        let txpool_transactions_mock = mock_daemon.mock_txpool_transactions(
            "tests/rpc_resources/transaction_hashes_with_payment_2.json",
            "tests/rpc_resources/transactions_with_payment_2.json",
        );

        // Payment 1 should be paid now.
        let update = subscriber_1
            .recv_timeout(Duration::from_millis(1000))
            .expect("failed to retrieve payment update");

        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 97));
        assert_eq!(update.amount_paid(), 74839140);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.current_height(), 2477661);
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), Some(0));

        // Payment 2 should not have an update.
        subscriber_2
            .recv_timeout(Duration::from_millis(1000))
            .expect_err("should not have received an update, but did");

        assert!(txpool_hashes_mock.hits() > 0);
        assert!(txpool_transactions_mock.hits() > 0);

        // Move forward a block.
        let txpool_hashes_mock =
            mock_daemon.mock_txpool_hashes("tests/rpc_resources/txpool_hashes.json");
        let height_mock = mock_daemon.mock_daemon_height(2477662);

        let update = subscriber_1
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 97));
        assert_eq!(update.amount_paid(), 74839140);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.current_height(), 2477662);
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), Some(1));

        let update = subscriber_2
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 138));
        assert_eq!(update.amount_paid(), 37419570);
        assert!(!update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.current_height(), 2477662);
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), None);

        assert!(txpool_hashes_mock.hits() > 0);
        assert!(height_mock.hits() > 0);

        // Move forward a block.
        let txpool_hashes_mock =
            mock_daemon.mock_txpool_hashes("tests/rpc_resources/txpool_hashes.json");
        let height_mock = mock_daemon.mock_daemon_height(2477663);

        let update = subscriber_1
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 97));
        assert_eq!(update.amount_paid(), 74839140);
        assert!(!update.is_expired());
        assert!(update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.current_height(), 2477663);
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), Some(2));

        let update = subscriber_2
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        assert_eq!(update.amount_requested(), 70000000);
        assert_eq!(update.index(), SubIndex::new(1, 138));
        assert_eq!(update.amount_paid(), 37419570);
        assert!(update.is_expired());
        assert!(!update.is_confirmed());
        assert_eq!(update.expiration_at() - update.started_at(), 7);
        assert_eq!(update.current_height(), 2477663);
        assert_eq!(update.confirmations_required(), 2);
        assert_eq!(update.confirmations(), None);

        assert!(txpool_hashes_mock.hits() > 0);
        assert!(height_mock.hits() > 0);
    })
}

#[test]
fn reproducible_seed() {
    // Setup.
    common::init_logger();
    let temp_dir = common::new_temp_dir();
    let mock_daemon = MockDaemon::new_mock_daemon();
    let rt = Runtime::new().expect("failed to create tokio runtime");

    // Create payment gateway pointing at temp directory and mock daemon.
    let payment_gateway =
        PaymentGatewayBuilder::new(common::PRIVATE_VIEW_KEY, common::PUBLIC_SPEND_KEY)
            .db_path(
                temp_dir
                    .path()
                    .to_str()
                    .expect("failed to get temporary directory path"),
            )
            // Faster scan rate so the update is received sooner.
            .scan_interval(Duration::from_millis(100))
            .daemon_url(&mock_daemon.url(""))
            .seed(1)
            .build();

    // Run it.
    rt.block_on(async {
        payment_gateway
            .run()
            .await
            .expect("failed to run payment gateway");

        // Add the payment.
        let mut subscriber = payment_gateway
            .new_payment(1, 5, 10)
            .await
            .expect("failed to add new payment to payment gateway for tracking");

        // Get initial update.
        let update = subscriber
            .recv_timeout(Duration::from_millis(2000))
            .expect("failed to retrieve payment update");

        // Check that it is as expected.
        assert_eq!(update.index(), SubIndex::new(1, 97));
    })
}
