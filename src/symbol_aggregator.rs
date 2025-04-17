use crate::api::StatsResult;
use crate::kahan::KahanSum;
use crate::monotonic_queue::MonotonicQueue;
use crate::shared_monotonic_queue::{MaxCmp, MinCmp, SharedMonotonicQueue};

pub struct SymbolAggregator {
    buffer: Vec<f64>,
    capacity: usize,
    head: usize,
    len: usize,
    index: u64,
    levels: [LevelStats; 8],
    minq: SharedMonotonicQueue<MinCmp>,
    maxq: SharedMonotonicQueue<MaxCmp>,
}

pub struct LevelStats {
    pub size: usize,
    pub count: usize,
    pub sum: KahanSum,
    pub sum_sq: KahanSum,
    // pub minq: MonotonicQueue<MinCmp>,
    // pub maxq: MonotonicQueue<MaxCmp>,
}

impl SymbolAggregator {
    pub fn new() -> Self {
        const MAX_K: usize = 8;
        const MAX_CAPACITY: usize = 2usize.pow(MAX_K as u32);
        let sizes = std::array::from_fn(|i| 2u64.pow((i + 1) as u32));

        Self {
            buffer: vec![0.0; MAX_CAPACITY],
            capacity: MAX_CAPACITY,
            head: 0,
            len: 0,
            index: 0,
            levels: std::array::from_fn(|i| {
                let size = 2usize.pow((i + 1) as u32);
                LevelStats {
                    size,
                    count: 0,
                    sum: KahanSum::new(),
                    sum_sq: KahanSum::new(),
                    // minq: MonotonicQueue::new(),
                    // maxq: MonotonicQueue::new(),
                }
            }),
            minq: SharedMonotonicQueue::<MinCmp>::new(sizes),
            maxq: SharedMonotonicQueue::<MaxCmp>::new(sizes),
        }
    }

    pub fn add_batch(&mut self, values: &[f64]) {
        for &value in values {
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

                    level.sum.sub(old_value);
                    level.sum_sq.sub(old_value * old_value);
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

            // tracing::info!("adding value: {value} @ {insert_index}");
            self.buffer[insert_index] = value;
            self.index += 1;

            for level in self.levels.iter_mut() {
                level.sum.add(value);
                level.sum_sq.add(value * value);
                level.count += 1;
                // level.minq.push(self.index - 1, value);
                // level.maxq.push(self.index - 1, value);
            }

            self.minq.push(self.index - 1, value);
            self.maxq.push(self.index - 1, value);
        }

        // for level in self.levels.iter_mut() {
        //     let window_start = self.index - (level.size.min(self.len) as u64);
        //     level.minq.evict_older_than(window_start);
        //     level.maxq.evict_older_than(window_start);
        // }

        self.minq.evict_older_than(self.index);
        self.maxq.evict_older_than(self.index);
    }

    pub fn get_stats(&mut self, k: u32) -> Option<StatsResult> {
        if !(1..=8).contains(&k) {
            return None;
        }

        let level = &self.levels[k as usize - 1];
        if level.count == 0 {
            return None;
        }

        // self.minq.refresh_best((k - 1) as usize, self.index);
        // self.maxq.refresh_best((k - 1) as usize, self.index);

        let n = level.count as f64;
        let sum = level.sum.get();
        let sum_sq = level.sum_sq.get();
        let avg = sum / n;
        let var = (sum_sq / n) - (avg * avg);
        let last = self.buffer[(self.head + self.len - 1) % self.capacity];
        // let min1 = level.minq.best()?;
        // let max1 = level.maxq.best()?;

        let min = self.minq.best((k - 1) as usize)?;
        let max = self.maxq.best((k - 1) as usize)?;

        // assert_eq!(min, min1);
        // assert_eq!(max, max1);

        tracing::info!("get_stats: count: {n} sum: {sum} for size: {}", level.size);
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
