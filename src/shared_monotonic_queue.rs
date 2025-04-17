use std::collections::VecDeque;

/// Strictly monotonic ring of values ordered by given `Comparator`.
///
/// Allows getting `min` or `max` value for the highest level in `O(1)` time.
/// Other levels have worst-case `O(log n)`, where `n` is number of entries in top level,
/// but have caching capability for constant time retrieval.
///
/// ## Design rationale
/// I was considering maintaining best indexes in `push` operation, but given that
/// batches size are at most `10^4` and highest level is `10^8`, we should expect much
/// more `push`es than statistic retrievals. So I decided to just invalidate best indexes
/// in `evict_older_than` and restore them on demand in `best_or_refresh`. This should
/// have much better amortised time complexity, than refreshing in `push`, or even after
/// a batch.
///
/// ## Impl note
/// Stores tuples of (logical_index, value) monotonically ordered.
/// Equal values are not stored. New equal value evicts old one.
///
/// Logical index is not reset. `u64::MAX` is big enough for the server to operate
/// for few hundred years, even under heavy load, until it will overflow.
pub struct SharedMonotonicQueue<C: Comparator, const LEVELS: usize, const RADIX: usize> {
    pub entries: VecDeque<(u64, f64)>,
    pub views: [LevelView; LEVELS], // we do not need last view, but LEVELS-1 would not compile
    _cmp: std::marker::PhantomData<C>,
}

/// Trait for comparing values in monotonic queues
pub trait Comparator {
    /// What `better` means for given stat.
    /// Used to decide weather to evict value from the deque back.
    fn better(new: f64, existing: f64) -> bool;

    /// for debugging
    #[allow(dead_code)]
    fn name() -> &'static str;
}

pub struct MinCmp;
pub struct MaxCmp;

/// `min` comparator
impl Comparator for MinCmp {
    fn better(new: f64, existing: f64) -> bool {
        new <= existing
    }

    fn name() -> &'static str {
        "min"
    }
}
/// `max` comparator
impl Comparator for MaxCmp {
    fn better(new: f64, existing: f64) -> bool {
        new >= existing
    }

    fn name() -> &'static str {
        "max"
    }
}

/// View for levels lower than the maximum one.
#[derive(Clone, Copy)]
pub struct LevelView {
    /// number of the level, used for debug
    id: usize,
    /// size of the window for given level
    pub window_size: u64,
    /// if `Some` it keeps the index of entry for best value in given level
    pub best_idx: Option<usize>,
}

impl<C: Comparator, const LEVELS: usize, const RADIX: usize>
    SharedMonotonicQueue<C, LEVELS, RADIX>
{
    pub fn new(window_sizes: [u64; LEVELS]) -> Self {
        Self {
            entries: VecDeque::new(),
            views: std::array::from_fn(|i| LevelView {
                id: i,
                window_size: window_sizes[i],
                best_idx: None,
            }),
            _cmp: std::marker::PhantomData,
        }
    }

    /// Pushes single value to the `deque` preserving strict monotonic invariant.
    ///
    /// Evicts (from the back) all values worse than given one.
    /// In case of eviction, updates or sets minimum index of evicted value.
    pub fn push(&mut self, index: u64, value: f64, min_evicted_idx: &mut Option<usize>) {
        let mut evicted = false;
        while let Some(&(_, back_val)) = self.entries.back() {
            if C::better(value, back_val) {
                self.entries.pop_back();
                evicted = true;
            } else {
                break;
            }
        }

        if evicted {
            let new_idx = self.entries.len();
            let min_evicted_idx = min_evicted_idx.get_or_insert(new_idx);
            if new_idx < *min_evicted_idx {
                *min_evicted_idx = new_idx;
            }
        }

        tracing::debug!(
            "{}, pushing: value: ({index}, {value}) @ {}",
            C::name(),
            self.entries.len()
        );
        self.entries.push_back((index, value));
    }

    /// Evicts values from the front of the `deque`, if are too old,
    /// based on `current_index` and top level window size.
    ///
    /// Invalidates best indexes for lower level views if needed, based on `min_evicted_idx`
    pub fn evict(&mut self, current_index: u64, min_evicted_idx: Option<usize>) {
        // first invalidate level best indexes cache if needed
        if let Some(min_evicted_idx) = min_evicted_idx {
            tracing::debug!(
                "{}, validating push-evicted best indexes before {min_evicted_idx}",
                C::name()
            );
            // we do not need to update last LEVEL, because it is full queue
            for view in self.views.iter_mut().take(LEVELS - 1) {
                if let Some(idx) = view.best_idx {
                    if idx < min_evicted_idx {
                        tracing::debug!(
                            "{}, invalidating push-evicted best index:{idx} level {}",
                            C::name(),
                            view.id,
                        );
                        view.best_idx = None;
                    }
                }
            }
        }

        // now evict to old values
        let max_window = RADIX.pow(LEVELS as u32) as u64;
        let oldest_allowed = current_index.saturating_sub(max_window);

        tracing::trace!(
            "{}, evicting older than: {oldest_allowed} out of {:?}",
            C::name(),
            self.entries
        );
        let mut front_evicted = 0usize;
        while let Some(&(idx, _)) = self.entries.front() {
            if idx < oldest_allowed {
                self.entries.pop_front();
                front_evicted += 1;
            } else {
                break;
            }
        }

        if front_evicted > 0 {
            tracing::debug!(
                "{}, evicted: {front_evicted} from front, validating too old best indexes: {:?}",
                C::name(),
                self.debug_best_indexes(),
            );
            // we do not need to update last LEVEL, because it is full queue
            for view in self.views.iter_mut().take(LEVELS - 1) {
                let min_index = current_index.saturating_sub(view.window_size);
                if let Some(ref mut idx) = view.best_idx {
                    *idx -= front_evicted;
                    if let Some((index, _)) = self.entries.get(*idx) {
                        if *index < min_index {
                            // invalidate best index as too old; will be set by `best_or_refresh`
                            tracing::debug!(
                                "{}, evicted: invalidating too old best index:{idx} level {}",
                                C::name(),
                                view.id,
                            );
                            view.best_idx = None;
                        }
                    }
                }
            }
        }
    }

    /// Gets best value for given level.
    ///
    /// Last level is special and has O(1) cost.
    /// Other levels are O(1) or O(log(n)) if best index was invalidated.
    pub fn best_or_refresh(&mut self, level: usize, current_index: u64) -> Option<f64> {
        if level == LEVELS - 1 {
            let front = self.entries.front();
            tracing::debug!(
                "{}, best: front: {:?} of {:?}",
                C::name(),
                front,
                self.entries
            );
            return front.map(|&(_, v)| v);
        }

        let view = &mut self.views[level];
        let min_index = current_index.saturating_sub(view.window_size);

        tracing::trace!(
            "{}, checking cached best index:{:?} level {level}",
            C::name(),
            view.best_idx,
        );
        if let Some(idx) = view.best_idx {
            if let Some((index, value)) = self.entries.get(idx) {
                if *index >= min_index {
                    tracing::debug!(
                        "{}, best: cached index:{idx} level {level}: {:?}",
                        C::name(),
                        self.entries
                    );
                    return Some(*value);
                }
            }
        }

        match self
            .entries
            .binary_search_by_key(&min_index, |&(idx, _)| idx)
        {
            Ok(idx) => view.best_idx = Some(idx),
            Err(idx) => view.best_idx = Some(idx),
        }

        tracing::debug!(
            "{}, best: index:{} level {level}: {:?}",
            C::name(),
            view.best_idx.expect("set just above"),
            self.entries
        );
        self.views[level]
            .best_idx
            .and_then(|i| self.entries.get(i).map(|&(_, v)| v))
    }

    #[allow(dead_code)]
    pub fn debug_best_indexes(&self) -> [Option<usize>; LEVELS] {
        std::array::from_fn(|i| self.views[i].best_idx)
    }
}
