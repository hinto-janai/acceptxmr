use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use log::{error, info, trace};
use monero::cryptonote::{hash::Hashable, onetime_key::SubKeyChecker};
use tokio::join;
use tokio::sync::Mutex;

use crate::AcceptXmrError;
use crate::{rpc::RpcClient, BlockCache, PaymentsDb, SubIndex, Transfer, TxpoolCache};

pub(crate) struct Scanner {
    payments_db: PaymentsDb,
    // Block cache and txpool cache are mutexed to allow concurrent block & txpool scanning. This is
    // necessary even though txpool scanning doesn't use the block cache, and vice versa, because
    // rust doesn't allow mutably borrowing only part of "self".
    block_cache: Mutex<BlockCache>,
    txpool_cache: Mutex<TxpoolCache>,
    first_scan: bool,
}

impl Scanner {
    pub async fn new(
        rpc_client: RpcClient,
        payments_db: PaymentsDb,
        block_cache_size: u64,
        atomic_height: Arc<AtomicU64>,
    ) -> Result<Scanner, AcceptXmrError> {
        // Determine sensible initial height for block cache.
        let height = match payments_db.lowest_height() {
            Ok(Some(h)) => {
                info!("Pending payments found in AcceptXMR database. Resuming from last block scanned: {}", h);
                h
            }
            Ok(None) => {
                let h = rpc_client.daemon_height().await?;
                info!("No pending payments found in AcceptXMR database. Skipping to blockchain tip: {}", h);
                h
            }
            Err(e) => {
                panic!("failed to determine suitable initial height for block cache from pending payments database: {}", e);
            }
        };

        // Set atomic height to the above determined initial height. This sets the height of the
        // main PaymentGateway as well.
        atomic_height.store(height, Ordering::Relaxed);

        // Initialize block cache and txpool cache.
        let (block_cache, txpool_cache) = join!(
            BlockCache::init(rpc_client.clone(), block_cache_size, atomic_height),
            TxpoolCache::init(rpc_client.clone())
        );

        Ok(Scanner {
            payments_db,
            block_cache: Mutex::new(block_cache?),
            txpool_cache: Mutex::new(txpool_cache?),
            first_scan: true,
        })
    }

    /// Scan for payment updates.
    pub async fn scan(&mut self, sub_key_checker: &SubKeyChecker<'_>) {
        // Update block cache, and scan both it and the txpool.
        let (blocks_amounts_or_err, txpool_amounts_or_err) = join!(
            self.scan_blocks(sub_key_checker),
            self.scan_txpool(sub_key_checker)
        );
        let height = self.block_cache.lock().await.height.load(Ordering::Relaxed);

        let blocks_amounts = match blocks_amounts_or_err {
            Ok(amts) => amts,
            Err(e) => {
                error!("Skipping scan! Encountered a problem while updating or scanning the block cache: {}", e);
                return;
            }
        };
        let txpool_amounts = match txpool_amounts_or_err {
            Ok(amts) => amts,
            Err(e) => {
                error!("Skipping scan! Encountered a problem while updating or scanning the txpool cache: {}", e);
                return;
            }
        };

        // Combine transfers into one big vec.
        let transfers: Vec<(SubIndex, Transfer)> = blocks_amounts
            .into_iter()
            .chain(txpool_amounts.into_iter())
            .collect();

        if self.first_scan {
            self.first_scan = false;
        }

        let deepest_update = transfers
            .iter()
            .min_by(|(_, transfer_1), (_, transfer_2)| transfer_1.cmp_by_age(transfer_2))
            .map_or(height + 1, |(_, transfer)| {
                transfer.height.unwrap_or(height + 1)
            });

        // A place to keep track of what payments are changing, so we can log updates later.
        let mut updated_payments = Vec::new();

        // Prepare updated payments.
        // TODO: Break this out into its own function.
        for payment_or_err in self.payments_db.iter() {
            // Retrieve old payment object.
            let old_payment = match payment_or_err {
                Ok(p) => p,
                Err(e) => {
                    error!(
                        "Failed to retrieve old payment object from database while iterating through database: {}", e
                    );
                    continue;
                }
            };
            let mut payment = old_payment.clone();

            // Remove transfers occurring later than the deepest block update.
            payment
                .transfers
                .retain(|transfer| transfer.older_than(deepest_update));

            // Add transfers from blocks and txpool.
            for (sub_index, owned_transfer) in &transfers {
                if sub_index == &payment.index && owned_transfer.newer_than(payment.started_at) {
                    payment.transfers.push(*owned_transfer);
                }
            }

            // Update payment's current_block.
            if payment.current_height != height {
                payment.current_height = height;
            }

            // No need to recalculate total paid_amount or paid_at unless something changed.
            if payment != old_payment {
                // Zero it out first.
                payment.paid_at = None;
                payment.amount_paid = 0;
                // Now add up the transfers.
                for transfer in &payment.transfers {
                    payment.amount_paid += transfer.amount;
                    if payment.amount_paid >= payment.amount_requested && payment.paid_at.is_none()
                    {
                        payment.paid_at = transfer.height;
                    }
                }

                // This payment has been updated. We can now add it in with the other
                // updated_payments.
                updated_payments.push(payment);
            }
        }

        // Save and log updates.
        for payment in &updated_payments {
            trace!(
                "Payment update for subaddress index {}: \
                    \n{}",
                payment.index,
                payment
            );
            if let Err(e) = self.payments_db.update(payment.index, payment) {
                error!(
                    "Failed to save update to payment for index {} to database: {}",
                    payment.index, e
                );
            }
        }

        // Flush changes to the database.
        self.payments_db.flush();
    }

    /// Update block cache and scan the blocks.
    ///
    /// Returns a vector of tuples of the form (subaddress index, amount, height)
    async fn scan_blocks(
        &self,
        sub_key_checker: &SubKeyChecker<'_>,
    ) -> Result<Vec<(SubIndex, Transfer)>, AcceptXmrError> {
        let mut block_cache = self.block_cache.lock().await;

        // Update block cache.
        let mut blocks_updated = block_cache.update().await?;

        // If this is the first scan, we want to scan all the blocks in the cache.
        if self.first_scan {
            blocks_updated = block_cache.blocks.len().try_into().unwrap();
        }

        let mut transfers = Vec::new();

        // Scan updated blocks.
        for i in (0..blocks_updated.try_into().unwrap()).rev() {
            let transactions = &block_cache.blocks[i].3;
            let amounts_received = self.scan_transactions(transactions, sub_key_checker)?;
            trace!(
                "Scanned {} transactions from block {}, and found {} transactions to tracked payments",
                transactions.len(),
                block_cache.blocks[i].1,
                amounts_received.len()
            );

            let height: u64 = block_cache.height.load(Ordering::Relaxed) - i as u64;

            // Add what was found into the list.
            transfers.extend::<Vec<(SubIndex, Transfer)>>(
                amounts_received
                    .into_iter()
                    .flat_map(|(_, amounts)| amounts)
                    .map(|amount| (amount.0, Transfer::new(amount.1, Some(height))))
                    .collect(),
            );
        }

        Ok(transfers)
    }

    /// Retrieve and scan transaction pool.
    ///
    /// Returns a vector of tuples of the form (subaddress index, amount)
    async fn scan_txpool(
        &self,
        sub_key_checker: &SubKeyChecker<'_>,
    ) -> Result<Vec<(SubIndex, Transfer)>, AcceptXmrError> {
        // Update txpool.
        let mut txpool_cache = self.txpool_cache.lock().await;
        let new_transactions = txpool_cache.update().await?;

        // Transfers previously discovered the txpool (no reason to scan the same transactions
        // twice).
        let discovered_transfers = txpool_cache.discovered_transfers();

        // Scan txpool.
        let amounts_received = self.scan_transactions(&new_transactions, sub_key_checker)?;
        trace!(
            "Scanned {} transactions from txpool, and found {} transfers for tracked payments",
            new_transactions.len(),
            amounts_received.len()
        );

        let new_transfers: HashMap<monero::Hash, Vec<(SubIndex, Transfer)>> = amounts_received
            .iter()
            .map(|(hash, amounts)| {
                (
                    *hash,
                    amounts
                        .iter()
                        .map(|(sub_index, amount)| (*sub_index, Transfer::new(*amount, None)))
                        .collect(),
                )
            })
            .collect();

        let mut transfers: HashMap<monero::Hash, Vec<(SubIndex, Transfer)>> = new_transfers.clone();
        // CLoning here because discovered_transactions is owned by the txpool cache.
        transfers.extend(discovered_transfers.clone());

        // Add the new transfers to the cache for next scan.
        txpool_cache.insert_transfers(&new_transfers);

        Ok(transfers
            .into_iter()
            .flat_map(|(_, amounts)| amounts)
            .collect())
    }

    fn scan_transactions(
        &self,
        transactions: &[monero::Transaction],
        sub_key_checker: &SubKeyChecker,
    ) -> Result<HashMap<monero::Hash, Vec<(SubIndex, u64)>>, AcceptXmrError> {
        let mut amounts_received = HashMap::new();
        for tx in transactions {
            // Scan transaction for owned outputs.
            let transfers = tx.check_outputs_with(sub_key_checker).unwrap();

            for transfer in &transfers {
                let sub_index = SubIndex::from(transfer.sub_index());

                // If this payment is being tracked, add the amount and payment ID to the result set.
                if self.payments_db.contains_key(sub_index)? {
                    let amount = transfers[0]
                        .amount()
                        .ok_or(AcceptXmrError::Unblind(sub_index))?;
                    amounts_received
                        .entry(tx.hash())
                        .or_insert_with(Vec::new)
                        .push((sub_index, amount));
                }
            }
        }

        Ok(amounts_received.into_iter().collect())
    }
}
