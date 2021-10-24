use std::{
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

use sled::Event;

use crate::{payments_db::PaymentStorageError, AcceptXmrError, Payment};

/// A means of receiving updates on a given payment. Subscribers are returned by [`PaymentGateways`](crate::PaymentGateway)
/// when a creating or subscribing to a payment.
pub struct Subscriber(sled::Subscriber);

impl Subscriber {
    pub(crate) fn new(sled_subscriber: sled::Subscriber) -> Subscriber {
        Subscriber(sled_subscriber)
    }

    /// Attempts to wait for a payment update from this subscriber.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed, or if there is an error deserializing the update.
    pub fn recv(&mut self) -> Result<Payment, AcceptXmrError> {
        let maybe_event = self.0.next();
        match maybe_event {
            Some(Event::Insert { value, .. }) => bincode::deserialize(&value)
                .map_err(|e| AcceptXmrError::from(PaymentStorageError::from(e))),
            Some(Event::Remove { .. }) => self.recv(),
            None => Err(AcceptXmrError::SubscriberRecv),
        }
    }

    /// Attempts to wait for a payment update from this subscriber, returning an error if no update
    /// arrives within the provided `Duration`.
    ///
    /// # Errors
    ///
    /// Returns an error if the channel is closed, if an update is not received in time, or if there
    /// is an error deserializing the update.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<Payment, AcceptXmrError> {
        let start = Instant::now();
        loop {
            let event_or_err = self.0.next_timeout(timeout - start.elapsed());
            match event_or_err {
                Ok(Event::Insert { value, .. }) => {
                    return bincode::deserialize(&value)
                        .map_err(|e| AcceptXmrError::from(PaymentStorageError::from(e)))
                }
                Ok(Event::Remove { .. }) => continue,
                Err(RecvTimeoutError::Timeout) => {
                    return Err(AcceptXmrError::SubscriberRecvTimeout)
                }
                Err(RecvTimeoutError::Disconnected) => return Err(AcceptXmrError::SubscriberRecv),
            }
        }
    }
}

impl Iterator for Subscriber {
    type Item = Result<Payment, AcceptXmrError>;

    fn next(&mut self) -> Option<Result<Payment, AcceptXmrError>> {
        // TODO: This shouldn't be using a timeout, but I am unaware of a better way to do it
        // given the limited options made available by sled.
        match self.0.next_timeout(Duration::from_nanos(0)) {
            Ok(Event::Insert { value, .. }) => Some(
                bincode::deserialize(&value)
                    .map_err(|e| AcceptXmrError::from(PaymentStorageError::from(e))),
            ),
            _ => None,
        }
    }
}
