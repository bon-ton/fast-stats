use dashmap::DashMap;
use std::sync::{LazyLock, Mutex};

use crate::symbol_aggregator::SymbolAggregator;

pub const MAX_K: usize = 8;
pub const RADIX: usize = 2;

/// There will NOT be concurrent requests for single symbol.
pub static SYMBOLS: LazyLock<DashMap<String, Mutex<SymbolAggregator<MAX_K, RADIX>>>> =
    LazyLock::new(DashMap::new);
