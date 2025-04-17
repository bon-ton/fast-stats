use crate::api::StatsResult;
use crate::kahan::NeumaierSum;
// use crate::monotonic_queue::MonotonicQueue;
use crate::shared_monotonic_queue::{MaxCmp, MinCmp, SharedMonotonicQueue};

/// The core of this service. Maintains all data per symbol to provide fast stats:
/// * cyclic buffer of values to get `last`, `avg` and `var`, shared for all levels
/// * two strictly monotonic deques to get `min` and `max`, shared for all levels
///
/// Space complexity is `O(n)` where `n` is top level size. Constant is small ~`2`:
/// * `1n` for all values, and amortised `1n` for `min` and `max` queues.
///
/// Adding batch has `O(n)` time complexity. Eviction from buffer is online,
/// as we are overwriting old values and need to update stats.
/// Eviction from deques is online for worse values, but for too old values it is once,
/// after all values from batch are pushed.
///
/// Potential improvement is to have buffer of slightly bigger capacity (max level + max batch),
/// to do the eviction once at the end as well.
///
/// Getting stats has
/// * `O(1)` complexity for all top level stats
/// * `O(1)` for `last`, `avg`, `var` stats regardless of the level  
/// * `O(log n)` pessimistic for lower levels `min` and `max` stats
///   * `O(1)` if cache is hit for lower levels `min` and `max`
///
/// Impl note:
/// Const generics are used to facilitate testing.
pub struct SymbolAggregator<const LEVELS: usize, const RADIX: usize> {
    /// ring of values
    buffer: Vec<f64>,
    /// capacity of the whole buffer (equals top level window size: `10^8`)
    capacity: usize,
    /// index of the `last` value inserted to the `buffer`
    ///
    /// fresh struct have it set to `capacity` which is logically `-1`
    tip: usize,
    /// number of values in the `buffer`
    len: usize,
    /// total number of elements added to the ring from the service start; never resets
    index: u64,
    /// Each level has own precomputed stats to get `avg` and `var` in `O(1)`
    levels: [LevelStats; LEVELS],
    /// Single ring of precomputed stats to get `min` in `O(1)` or `O(log n)`
    minq: SharedMonotonicQueue<MinCmp, LEVELS, RADIX>,
    /// Ditto, just for `max`
    maxq: SharedMonotonicQueue<MaxCmp, LEVELS, RADIX>,
}

/// Maintains sum of values and their squares for fast `avg` and `var` stats at single level.
pub struct LevelStats {
    /// for debug
    id: usize,
    /// size of the level window
    pub size: usize,
    /// number of elements currently in the level
    pub count: usize,
    /// sum of those elements
    pub sum: NeumaierSum,
    /// sum of square roots of those elements
    pub sum_sq: NeumaierSum,
    // pub minq: MonotonicQueue<MinCmp>,
    // pub maxq: MonotonicQueue<MaxCmp>,
}

impl LevelStats {
    fn is_full(&self) -> bool {
        self.count == self.size
    }

    /// Push new value (and its square root) to this level stats, possibly evicting `oldest_value`.
    fn push(&mut self, val: f64, val_sq: f64, oldest_value: f64) {
        self.evict_oldest(oldest_value);
        self.count += 1;
        self.sum += val;
        self.sum_sq += val_sq;
    }

    /// Evicts oldest value from level stats if full, as a preparation to push new value.
    fn evict_oldest(&mut self, oldest_value: f64) {
        if !self.is_full() {
            // nothing to do
            return;
        }

        tracing::trace!("evicting oldest value {oldest_value} of level {}", self.id);
        self.sum += -oldest_value;
        self.sum_sq += -(oldest_value * oldest_value);
        self.count = self.count.saturating_sub(1);
    }
}

impl<const LEVELS: usize, const RADIX: usize> SymbolAggregator<LEVELS, RADIX> {
    pub fn new() -> Self {
        let capacity = RADIX.pow(LEVELS as u32);
        let sizes = std::array::from_fn(|i| (RADIX as u64).pow((i + 1) as u32));

        Self {
            buffer: vec![0.0; capacity],
            capacity,
            tip: capacity, // logically -1
            len: 0,
            index: 0,
            levels: std::array::from_fn(|i| {
                let size = RADIX.pow((i + 1) as u32);
                LevelStats {
                    id: i,
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
        tracing::debug!("add_batch: {values:?}");

        let mut min_minq_evicted_idx = None;
        let mut min_maxq_evicted_idx = None;

        for &val in values {
            if !self.try_push(val) {
                continue;
            }
            self.minq.push(self.index, val, &mut min_minq_evicted_idx);
            self.maxq.push(self.index, val, &mut min_maxq_evicted_idx);
            self.index += 1;
        }

        // for level in self.levels.iter_mut() {
        //     let window_start = self.index - (level.size.min(self.len) as u64);
        //     level.minq.evict_older_than(window_start);
        //     level.maxq.evict_older_than(window_start);
        // }

        // eviction after adding whole batch
        self.minq.evict(self.index, min_minq_evicted_idx);
        self.maxq.evict(self.index, min_maxq_evicted_idx);
    }

    /// Tries to push single `val` to the ring and all level stats for `avg` and `var`.
    ///
    /// Potentially evicting the oldest value for each level,
    /// and overriding for top level, if buffer is full.
    ///
    /// Shifts `tip` and, if buffer is not full, increases `len`.
    ///
    /// Returns weather push was successful: might not be if value or sum of squares is too big.
    fn try_push(&mut self, val: f64) -> bool {
        let val_sq = val * val;
        let max_sum_sq = (self.levels[LEVELS - 1].sum_sq.clone() + val_sq).sum();
        if max_sum_sq.is_nan() || max_sum_sq.is_infinite() {
            tracing::warn!("ignoring {val} since its square root brings sum to {max_sum_sq}");
            return false;
        }

        let tip_plus_cap = self.tip + self.capacity;
        for level in self.levels.iter_mut() {
            let oldest_level_value = if level.is_full() {
                // tip: 13
                // level size: 10
                // offset: 4

                // tip: 1
                // level size: 1000
                // offset: 99_999_002
                let oldest_level_idx = (tip_plus_cap - level.size + 1) % self.capacity;
                self.buffer[oldest_level_idx]
            } else {
                0.
            };
            level.push(val, val_sq, oldest_level_value);
        }

        if !self.is_full() {
            self.len += 1;
        }
        self.tip = (self.tip + 1) % self.capacity;
        tracing::trace!("adding value: {val} @ {} / {}", self.tip, self.capacity);
        self.buffer[self.tip] = val;
        true
    }

    fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    /// returns the `last` inserted value to the ring, if any
    fn get_last(&mut self) -> Option<f64> {
        if self.len > 0 {
            return Some(self.buffer[self.tip]);
        }
        None
    }

    /// Get stats for given level `k`.
    ///
    /// We might hit infinity when calculating variance. In such a case `var` will
    /// be `null` in response. Later when too big values are evicted, `var` will be
    /// returned again.
    pub fn get_stats(&mut self, k: u32) -> Option<StatsResult> {
        let Some(last) = self.get_last() else {
            return None;
        };

        let k = k as usize;
        if !(1..=LEVELS).contains(&k) {
            return None;
        }

        let level = &self.levels[k - 1];

        // self.minq.refresh_best(k - 1, self.index);
        // self.maxq.refresh_best(k - 1, self.index);

        let n = level.count as f64;
        let sum = level.sum.sum();
        let sum_sq = level.sum_sq.sum();
        let avg = sum / n;
        let var = (sum_sq / n) - (avg * avg);
        if var.is_infinite() || var.is_nan() {
            tracing::warn!("variance not available: it is {var}");
        }
        // let min1 = level.minq.best()?;
        // let max1 = level.maxq.best()?;

        let min = self.minq.best_or_refresh(k - 1, self.index)?;
        let max = self.maxq.best_or_refresh(k - 1, self.index)?;

        // assert_eq!(min, min1);
        // assert_eq!(max, max1);

        tracing::debug!("get_stats: count: {n} sum: {sum} for level: {}", level.id);
        tracing::trace!(
            "get_stats: min best indexes: {:?}",
            self.minq.debug_best_indexes()
        );
        tracing::trace!(
            "get_stats: max best indexes: {:?}",
            self.maxq.debug_best_indexes()
        );

        Some(StatsResult {
            min,
            max,
            last,
            avg,
            var,
        })
    }
}
