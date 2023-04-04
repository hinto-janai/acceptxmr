//! # `AcceptXMR-Server`: A monero payment gateway.
//! `AcceptXMR-Server` is a batteries-included monero payment gateway built
//! around the `AcceptXMR` library.
//!
//! If your application requires more flexibility than `AcceptXMR-Server`
//! offers, please see the [`AcceptXMR`](../library/) library instead.

#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

mod api;
mod config;
mod logging;
mod websocket;

use acceptxmr::{storage::stores::Sqlite, PaymentGatewayBuilder};
use actix_session::{
    config::CookieContentSecurity, storage::CookieSessionStore, SessionMiddleware,
};
use actix_web::{cookie, web::Data, App, HttpServer};
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};

use crate::{
    api::{external, internal},
    config::read_config,
    logging::init_logger,
};

/// Length in bytes of secure session key for cookies.
const SESSION_KEY_LEN: usize = 64;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = read_config().unwrap();
    init_logger(config.logging);

    std::fs::create_dir_all(&config.database.path).expect("failed to create DB dir");
    let db_path = config
        .database
        .path
        .canonicalize()
        .expect("could not determine absolute database path")
        .join("database");
    let db_path_str = db_path.to_str().expect("failed to cast DB path to string");

    // The private view key should be stored securely outside of the git repository.
    // It is hardcoded here for demonstration purposes only.
    let private_view_key = "ad2093a5705b9f33e6f0f0c1bc1f5f639c756cdfc168c8f2ac6127ccbdab3a03";
    // No need to keep the primary address secret.
    let primary_address = "4613YiHLM6JMH4zejMB2zJY5TwQCxL8p65ufw8kBP5yxX9itmuGLqp1dS4tkVoTxjyH3aYhYNrtGHbQzJQP5bFus3KHVdmf";

    let invoice_store = Sqlite::new(db_path_str, "invoices").expect("failed to open invoice store");
    let payment_gateway = PaymentGatewayBuilder::new(
        private_view_key.to_string(),
        primary_address.to_string(),
        invoice_store,
    )
    .daemon_url("http://xmr-node.cakewallet.com:18081".to_string())
    .build()
    .expect("failed to build payment gateway");
    info!("Payment gateway created.");

    payment_gateway
        .run()
        .await
        .expect("failed to run payment gateway");
    info!("Payment gateway running.");

    // Watch for invoice updates and deal with them accordingly.
    let gateway_copy = payment_gateway.clone();
    std::thread::spawn(move || {
        // Watch all invoice updates.
        let mut subscriber = gateway_copy.subscribe_all();
        loop {
            let Some(invoice) = subscriber.blocking_recv() else { panic!("Blockchain scanner crashed!") };
            // If it's confirmed or expired, we probably shouldn't bother tracking it
            // anymore.
            if (invoice.is_confirmed() && invoice.creation_height() < invoice.current_height())
                || invoice.is_expired()
            {
                debug!(
                    "Invoice to index {} is either confirmed or expired. Removing invoice now",
                    invoice.index()
                );
                if let Err(e) = gateway_copy.remove_invoice(invoice.id()) {
                    error!("Failed to remove fully confirmed invoice: {}", e);
                };
            }
        }
    });

    // Create secure session key for cookies.
    let mut key_arr = [0u8; SESSION_KEY_LEN];
    thread_rng().fill(&mut key_arr[..]);
    let session_key = cookie::Key::generate();

    // Run the demo webpage.
    let shared_payment_gateway = Data::new(payment_gateway);
    HttpServer::new(move || {
        App::new()
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), session_key.clone())
                    .cookie_name("acceptxmr_session".to_string())
                    .cookie_secure(false)
                    .cookie_content_security(CookieContentSecurity::Private)
                    .build(),
            )
            .app_data(shared_payment_gateway.clone())
            .configure(external)
            .configure(internal)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
