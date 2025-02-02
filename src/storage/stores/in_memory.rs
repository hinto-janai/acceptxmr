use std::collections::{
    btree_map::{self, Entry},
    BTreeMap,
};

use thiserror::Error;

use crate::{storage::InvoiceStorage, Invoice, InvoiceId, SubIndex};

/// In-memory store of pending invoices. Note that invoices stored in memory
/// will not be recoverable on power loss.
pub struct InMemory(BTreeMap<InvoiceId, Invoice>);

impl InMemory {
    /// Create a new in-memory invoice store.
    #[must_use]
    pub fn new() -> InMemory {
        InMemory(BTreeMap::new())
    }
}

impl Default for InMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl InvoiceStorage for InMemory {
    type Error = InMemoryStorageError;
    type Iter<'a> = InMemoryIter<'a>;

    fn insert(&mut self, invoice: Invoice) -> Result<(), Self::Error> {
        if self.0.contains_key(&invoice.id()) {
            return Err(InMemoryStorageError::DuplicateEntry);
        }
        self.0.insert(invoice.id(), invoice);
        Ok(())
    }

    fn remove(&mut self, invoice_id: InvoiceId) -> Result<Option<Invoice>, Self::Error> {
        Ok(self.0.remove(&invoice_id))
    }

    fn update(&mut self, invoice: Invoice) -> Result<Option<Invoice>, Self::Error> {
        if let Entry::Occupied(mut entry) = self.0.entry(invoice.id()) {
            return Ok(Some(entry.insert(invoice)));
        }
        Ok(None)
    }

    fn get(&self, invoice_id: InvoiceId) -> Result<Option<Invoice>, Self::Error> {
        Ok(self.0.get(&invoice_id).cloned())
    }

    fn contains_sub_index(&self, sub_index: SubIndex) -> Result<bool, Self::Error> {
        Ok(self
            .0
            .range(InvoiceId::new(sub_index, 0)..)
            .next()
            .is_some())
    }

    fn try_iter(&self) -> Result<Self::Iter<'_>, InMemoryStorageError> {
        let iter = self.0.values();
        Ok(InMemoryIter(iter))
    }
}

pub struct InMemoryIter<'a>(btree_map::Values<'a, InvoiceId, Invoice>);

impl<'a> Iterator for InMemoryIter<'a> {
    type Item = Result<Invoice, InMemoryStorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|v| Ok(v.clone()))
    }
}

/// An error occurring while storing or retrieving pending invoices in memory.
#[derive(Error, Debug)]
#[error("in-memory invoice storage error")]
pub enum InMemoryStorageError {
    /// Attempted to insert an invoice which already exists
    #[error("attempted to insert an invoice which already exists")]
    DuplicateEntry,
}
