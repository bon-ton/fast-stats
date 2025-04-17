#[cfg(test)]
mod tests {
    use crate::symbol_aggregator::SymbolAggregator;

    #[test]
    fn test_rolling_stats() {
        let mut agg = SymbolAggregator::new();
        agg.add_batch(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        let stats = agg.get_stats(1).unwrap();
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.last, 5.0);
        assert!((stats.avg - 3.0).abs() < 1e-9);
        assert!((stats.var - 2.0).abs() < 1e-9);
    }
}
