use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub enum TransactionStatus { Active, Committed, Aborted, }
#[derive(Debug)]
pub struct TransactionItems {
    pub status: TransactionStatus,
    pub socket_addr: SocketAddr,
    pub last_txd: u32,
    pub modified_keys: Vec<u32>,
}
#[derive(Debug)]
pub struct Transaction {
    pub items: HashMap<u32, TransactionItems>,
    pub ip_txd: HashMap<SocketAddr, u32>,
}