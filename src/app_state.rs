use dashmap::DashMap;
use std::sync::{LazyLock, Mutex};

use crate::symbol_aggregator::SymbolAggregator;

pub static SYMBOLS: LazyLock<DashMap<String, Mutex<SymbolAggregator>>> =
    LazyLock::new(DashMap::new);
