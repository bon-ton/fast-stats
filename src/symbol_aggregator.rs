use crate::api::StatsResult;
use crate::kahan::NeumaierSum;
// use crate::monotonic_queue::MonotonicQueue;
use crate::shared_monotonic_queue::{MaxCmp, MinCmp, SharedMonotonicQueue};

pub struct SymbolAggregator<const LEVELS: usize, const RADIX: usize> {
    buffer: Vec<f64>,
    capacity: usize,
    head: usize,
    len: usize,
    index: u64,
    levels: [LevelStats; LEVELS],
    minq: SharedMonotonicQueue<MinCmp, LEVELS, RADIX>,
    maxq: SharedMonotonicQueue<MaxCmp, LEVELS, RADIX>,
}

pub struct LevelStats {
    pub size: usize,
    pub count: usize,
    pub sum: NeumaierSum,
    pub sum_sq: NeumaierSum,
    // pub minq: MonotonicQueue<MinCmp>,
    // pub maxq: MonotonicQueue<MaxCmp>,
}

impl<const LEVELS: usize, const RADIX: usize> SymbolAggregator<LEVELS, RADIX> {
    pub fn new() -> Self {
        let capacity = RADIX.pow(LEVELS as u32);
        let sizes = std::array::from_fn(|i| (RADIX as u64).pow((i + 1) as u32));

        Self {
            buffer: vec![0.0; capacity],
            capacity,
            head: 0,
            len: 0,
            index: 0,
            levels: std::array::from_fn(|i| {
                let size = RADIX.pow((i + 1) as u32);
                LevelStats {
                    size,
                    count: 0,
                    sum: 0f64.into(),
                    sum_sq: 0f64.into(),
                    // minq: MonotonicQueue::new(),
                    // maxq: MonotonicQueue::new(),
                }
            }),
            minq: SharedMonotonicQueue::<MinCmp, LEVELS, RADIX>::new(sizes),
            maxq: SharedMonotonicQueue::<MaxCmp, LEVELS, RADIX>::new(sizes),
        }
    }

    /// Add values to the batch.
    ///
    /// We skip values which square root are too big (infinity).
    pub fn add_batch(&mut self, values: &[f64]) {
        for &val in values {
            // tracing::info!("adding value: {value} @ {insert_index}");
            let val_sq = val * val;
            let max_sum_sq = (self.levels[LEVELS - 1].sum_sq.clone() + val_sq).sum();
            if max_sum_sq.is_nan() || max_sum_sq.is_infinite() {
                tracing::warn!("ignoring {val} since its square root brings sum to {max_sum_sq}");
                continue;
            }

            for level in self.levels.iter_mut() {
                while level.count >= level.size {
                    // let evicted_index = self.index - level.size as u64;
                    let offset = self.len - level.size;
                    let buf_idx = (self.head + offset) % self.capacity;
                    let old_value = self.buffer[buf_idx];
                    // tracing::info!(
                    //     "evicting value: {old_value} @ {buf_idx} logical: {evicted_index}/{} for level {}",
                    //     self.index,
                    //     level.size,
                    // );

                    level.sum += -old_value;
                    level.sum_sq += -(old_value * old_value);
                    level.count -= 1;
                }
            }

            let will_overwrite = self.len == self.capacity;

            let insert_index = if will_overwrite {
                let idx = self.head;
                self.head = (self.head + 1) % self.capacity;
                idx
            } else {
                let idx = (self.head + self.len) % self.capacity;
                self.len += 1;
                idx
            };

            self.buffer[insert_index] = val;
            self.minq.push(self.index, val);
            self.maxq.push(self.index, val);
            self.index += 1;

            for level in self.levels.iter_mut() {
                level.sum += val;
                level.sum_sq += val_sq;
                // tracing::info!("sum_sq for {} is {}", level.size, level.sum_sq.sum());
                level.count += 1;
                // level.minq.push(self.index - 1, value);
                // level.maxq.push(self.index - 1, value);
            }
        }

        // for level in self.levels.iter_mut() {
        //     let window_start = self.index - (level.size.min(self.len) as u64);
        //     level.minq.evict_older_than(window_start);
        //     level.maxq.evict_older_than(window_start);
        // }

        self.minq.evict_older_than(self.index);
        self.maxq.evict_older_than(self.index);
    }

    /// Get stats for given level `k`.
    ///
    /// We might hit infinity when calculating variance. In such a case `var` will
    /// be `null` in response. Later when too big values are evicted, `var` will be
    /// returned again.
    pub fn get_stats(&mut self, k: u32) -> Option<StatsResult> {
        if !(1..=LEVELS as u32).contains(&k) {
            return None;
        }

        let level = &self.levels[k as usize - 1];
        if level.count == 0 {
            return None;
        }

        // self.minq.refresh_best((k - 1) as usize, self.index);
        // self.maxq.refresh_best((k - 1) as usize, self.index);

        let n = level.count as f64;
        let sum = level.sum.sum();
        let sum_sq = level.sum_sq.sum();
        let avg = sum / n;
        let var = (sum_sq / n) - (avg * avg);
        if var.is_infinite() || var.is_nan() {
            tracing::warn!("variance not available: it is {var}");
        }
        let last = self.buffer[(self.head + self.len - 1) % self.capacity];
        // let min1 = level.minq.best()?;
        // let max1 = level.maxq.best()?;

        let min = self.minq.best_or_refresh((k - 1) as usize, self.index)?;
        let max = self.maxq.best_or_refresh((k - 1) as usize, self.index)?;

        // assert_eq!(min, min1);
        // assert_eq!(max, max1);

        // tracing::info!("get_stats: count: {n} sum: {sum} for size: {}", level.size);
        // tracing::info!(
        //     "get_stats: min best indexes: {:?}",
        //     self.minq.debug_best_indexes()
        // );
        // tracing::info!(
        //     "get_stats: max best indexes: {:?}",
        //     self.maxq.debug_best_indexes()
        // );
        // let end = (self.head + self.len) % self.capacity;
        // let beg = (end + self.capacity - level.count) % self.capacity;
        // tracing::info!("values: {:?}", &self.buffer[beg..end]);

        Some(StatsResult {
            min,
            max,
            last,
            avg,
            var,
        })
    }
}
