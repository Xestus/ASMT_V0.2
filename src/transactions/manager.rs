use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use crate::transactions::transactions::{Transaction, TransactionStatus};

pub fn get_oldest_active_txd(current_transactions: Arc<RwLock<Transaction>>, all_addr: Arc<RwLock<Vec<SocketAddr>>>) -> Option<u32> {
    let mut all_txd = Vec::new();

    {
        let tx = current_transactions.read().unwrap();
        let addr_read = all_addr.read().unwrap();

        // takes all the active transaction
        for i in addr_read.iter() {
            if let Some(x) =  tx.ip_txd.get(i) {
                if let Some(t_items) = tx.items.get(x) {
                    if t_items.status == TransactionStatus::Active {
                        all_txd.push(*x);
                    }
                }
            }
        }
    }

    all_txd.sort();

    all_txd.first().copied()

}
