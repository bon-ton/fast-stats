#[cfg(test)]
mod tests {
    use crate::symbol_aggregator::SymbolAggregator;
    use std::mem;

    #[test]
    fn test_small_stats() {
        let mut agg: SymbolAggregator<4, 2> = SymbolAggregator::new();
        agg.add_batch(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        // two last elems
        let stats = agg.get_stats(1).unwrap();
        assert_eq!(stats.min, 4.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.last, 5.0);
        assert_eq!(stats.avg, 4.5);
        assert_eq!(stats.var, 0.25);

        // four last elems
        let stats = agg.get_stats(2).unwrap();
        assert_eq!(stats.min, 2.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.last, 5.0);
        assert_eq!(stats.avg, 3.5);
        assert_eq!(stats.var, 1.25);

        // full set
        let stats = agg.get_stats(3).unwrap();
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.last, 5.0);
        assert_eq!(stats.avg, 3.0);
        assert_eq!(stats.var, 2.0);

        // last level
        let stats = agg.get_stats(4).unwrap();
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.last, 5.0);
        assert_eq!(stats.avg, 3.0);
        assert_eq!(stats.var, 2.0);
    }

    #[test]
    fn test_inf_values_skipped() {
        let mut agg: SymbolAggregator<2, 2> = SymbolAggregator::new();
        agg.add_batch(&[1e200, 1., 2.]);

        // two last elems
        let stats = agg.get_stats(1).unwrap();
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 2.0);
        assert_eq!(stats.last, 2.0);
        assert_eq!(stats.avg, 1.5);
        assert_eq!(stats.var, 0.25);

        // full set, too big value skipped
        let stats = agg.get_stats(2).unwrap();
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 2.0);
        assert_eq!(stats.last, 2.0);
        assert_eq!(stats.avg, 1.5);
        assert_eq!(stats.var, 0.25);
    }

    #[test]
    fn test_inf_variance() {
        let mut agg: SymbolAggregator<2, 2> = SymbolAggregator::new();
        agg.add_batch(&[1e154, -1e154]);

        // two last elems
        let stats = agg.get_stats(1).unwrap();
        assert_eq!(stats.min, 1e154);
        assert_eq!(stats.max, 1e154);
        assert_eq!(stats.last, 1e154);
        assert_eq!(stats.avg, 1e154);
        assert_eq!(stats.var, 0.0);

        let seriaized_stats = serde_json::ser::to_string(&stats).unwrap();
        assert_eq!(
            seriaized_stats,
            "{\"min\":1e154,\"max\":1e154,\"last\":1e154,\"avg\":1e154,\"var\":0.0}"
        );

        // full set
        let stats = agg.get_stats(2).unwrap();
        assert_eq!(stats.min, 1e154);
        assert_eq!(stats.max, 1e154);
        assert_eq!(stats.last, 1e154);
        assert_eq!(stats.avg, 1e154);
        assert_eq!(stats.var, 0.0);
    }

    #[test]
    fn test_max_variance() {
        tracing_subscriber::fmt::init();
        let mut agg: SymbolAggregator<2, 2> = SymbolAggregator::new();
        agg.add_batch(&[1e153, -1e153, 1e153]);

        // two last elems
        let stats = agg.get_stats(1).unwrap();
        assert_eq!(stats.min, -1e153);
        assert_eq!(stats.max, 1e153);
        assert_eq!(stats.last, 1e153);
        assert_eq!(stats.avg, 0.0);
        assert_eq!(stats.var, 1e306);

        // full set
        let stats = agg.get_stats(2).unwrap();
        assert_eq!(stats.min, -1e153);
        assert_eq!(stats.max, 1e153);
        assert_eq!(stats.last, 1e153);
        assert_eq!(stats.avg, 3.3333333333333333e152);
        assert_eq!(stats.var, 8.888888888888889e305);
    }

    #[test]
    fn test_skip_too_big_value_and_second() {
        let mut agg: SymbolAggregator<8, 2> = SymbolAggregator::new();
        let data = [
            f64::MAX, // this will be skipped
            1e154,    // this will be handled normally
            -357253.73,
            208516.47,
            414829.86,
            402216.5,
            167329.31,
            -183024.63,
            549964.39,
            569438.84,
            979579.66,
            1071457.96,
            973600.61,
            345601.07,
            -696843.71,
            -548727.68,
            -173981.1,
            -21233.47,
            -285452.15,
            -1081978.16,
            -1723903.72,
            -1860961.38,
            -2511160.49,
            -2149153.61,
            -2557439.23,
            -2523536.86,
            -2539620.89,
            -3499678.38,
            -2882446.72,
            -2435662.46,
            -1733641.21,
            -1298583.06,
            -1140037.51,
            -1651961.75,
            -1876330.13,
            -1854202.27,
            -2314838.9,
            -1784259.14,
            -2066958.79,
            -3278060.55,
            -4016425.44,
            -4008918.43,
            -4533880.31,
            -4276529.0,
            -4875035.33,
            -4486083.63,
            -4103970.57,
            -3886785.12,
            -4403644.4,
            -4719383.2,
            -4585644.0,
            -3881833.41,
            -3095630.32,
            -2479345.26,
            -1746200.96,
            -1795116.28,
            -1595478.84,
            -1699732.35,
            -1514387.81,
            -860798.93,
            25656.74,
            529841.08,
            876942.44,
            979566.09,
            936960.58,
            983303.17,
            383112.11,
            -58114.75,
            182779.9,
            -411397.95,
            -925727.59,
            106950.75,
            75258.92,
            153305.48,
            158622.11,
            271908.52,
            907157.24,
            790027.39,
            1029815.93,
            909729.21,
            547845.45,
            584871.03,
            -419290.33,
            -503000.06,
            439589.56,
            207975.42,
            -1062825.53,
            -807755.29,
            -1357822.64,
            -1748821.86,
            -2065145.63,
            -1247578.82,
            -781293.76,
            -1101851.44,
            -902415.02,
            -378487.5,
            -409150.91,
            309590.41,
            780656.35,
            747540.8,
            5826.69,
            -811943.08,
            -979471.61,
            -1138243.5,
            -721763.51,
            -228362.47,
            -148716.59,
            -665296.46,
            -1035627.69,
            -464671.03,
            -983069.72,
            -509832.29,
            185219.85,
            650326.2,
            810830.32,
            353665.57,
            -536024.93,
            376220.51,
            773798.54,
            988969.28,
            325287.81,
            187097.05,
            -382651.86,
            -271644.5,
            -144437.29,
            -64989.0,
            135690.87,
            260048.7,
            1469001.64,
            1978545.7,
            1930292.02,
            2113965.63,
            1634816.1,
            1619958.69,
            961033.7,
            514645.53,
            575992.68,
            1195093.34,
            968584.69,
            1006877.99,
            958567.93,
            584459.49,
            379057.74,
            1151803.61,
            784830.05,
            1405933.28,
            1823718.24,
            2696115.49,
            2492708.2,
            2397471.16,
            2076420.91,
            2177363.39,
            1313144.83,
            1305239.16,
            1395526.08,
            1410287.14,
            1652908.84,
            1970915.8,
            1444376.63,
            1308322.94,
            1216897.21,
            1704257.7,
            2674876.61,
            3000090.74,
            2875780.55,
            3281707.12,
            3185485.34,
            3052892.06,
            2404011.35,
            1764667.46,
            1213394.79,
            1504460.65,
            919759.46,
            431428.75,
            657336.81,
            1153777.7,
            1113863.69,
            1427595.71,
            1600084.25,
            2360601.35,
            2560209.46,
            2450245.8,
            2189858.13,
            1605624.59,
            1249879.72,
            1281352.81,
            452666.85,
            637661.7,
            860314.57,
            1585049.95,
            1237625.76,
            893584.43,
            620060.29,
            1105516.22,
            1582564.16,
            2495109.43,
            2357369.78,
            1816901.67,
            1557488.56,
            989491.3,
            1497477.35,
            1331464.94,
            1897653.93,
            1071943.72,
            412919.07,
            -287200.23,
            -348960.11,
            226027.04,
            -496911.39,
            -1472184.48,
            -1290916.62,
            -1393882.69,
            -1979183.93,
            -2344555.71,
            -1823798.78,
            -719619.72,
            -1055367.46,
            -509529.65,
            -637084.49,
            -120127.54,
            -912838.79,
            -564488.48,
            -648213.92,
            -134950.3,
            584531.4,
            743442.59,
            495836.97,
            95297.19,
            -179482.79,
            -753554.24,
            -752644.14,
            -489212.93,
            379500.83,
            -117890.0,
            -473419.04,
            -438476.85,
            -711508.04,
            -70749.78,
            346233.6,
            -83350.78,
            -28750.66,
            112963.61,
            587544.7,
            1070523.04,
            1484748.39,
            1105610.08,
            1128778.32,
            1445190.55,
            846673.38,
            1562243.65,
            711978.06,
            535744.92,
            687674.16,
            513530.44,
            607013.55,
            379221.31,
            702522.54,
        ];

        agg.add_batch(&data);
        let stats = agg.get_stats(8).unwrap();
        assert_eq!(stats.min, -4875035.33);
        assert_eq!(stats.max, 1e154);
        assert_eq!(stats.last, 702522.54);
        assert_eq!(stats.avg, 3.90625e151);
        assert_eq!(stats.var, 3.8909912109375e305);

        // skip biggest value 1e154 from the start
        agg.add_batch(&[928602.78]);
        let stats = agg.get_stats(8).unwrap();
        assert_eq!(stats.min, -4875035.33);
        assert_eq!(stats.max, 3281707.12);
        assert_eq!(stats.last, 928602.78);
        assert_eq!(stats.avg, 12558.220820312305);
        assert_eq!(stats.var, 2610076991714.1025);
    }

    #[test]
    fn test_big_stats() {
        let mut agg: SymbolAggregator<8, 2> = SymbolAggregator::new();
        let data = super::generate_random_data(257, 3.14, 271.72, 457325.);
        agg.add_batch(&data);

        // two last elems
        let stats = agg.get_stats(1).unwrap();
        let mut sec2last = data[data.len().wrapping_sub(2)];
        let mut last = data[data.len().wrapping_sub(1)];

        assert_eq!(stats.last, last);

        if last < sec2last {
            mem::swap(&mut last, &mut sec2last);
        }

        assert_eq!(stats.min, sec2last);
        assert_eq!(stats.max, last);
        assert!((stats.avg - (last + sec2last) / 2.).abs() < 2e-9);
        assert!((stats.var - (last - (last + sec2last) / 2.).powi(2)).abs() < 2e-1);

        // full stats
        let stats = agg.get_stats(8).unwrap();
        // first should be skipped
        let d = &data[1..];
        println!("{data:?}");
        assert_eq!(stats.min, d.to_vec().into_iter().reduce(f64::min).unwrap());
        assert_eq!(stats.max, d.to_vec().into_iter().reduce(f64::max).unwrap());
        assert_eq!(stats.last, data[data.len().wrapping_sub(1)]);
    }
}

#[cfg(feature = "test")]
pub fn generate_random_data(n: usize, base: f64, drift: f64, volatility: f64) -> Vec<f64> {
    use rand_distr::{Distribution, Normal};
    let mut rng = rand::rng();
    let normal = Normal::new(drift, volatility).unwrap();

    let mut prices = Vec::with_capacity(n);
    let mut price = base;
    for _ in 0..n {
        let delta = normal.sample(&mut rng);
        price += delta;
        prices.push((price * 100.0).round() / 100.0); // round to 2 decimals
    }

    prices
}
