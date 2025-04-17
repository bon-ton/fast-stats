use std::collections::VecDeque;

/// Trait for comparing values in monotonic queues
pub trait Comparator {
    fn better(new: f64, existing: f64) -> bool;

    fn name() -> &'static str;
}

pub struct MinCmp;
pub struct MaxCmp;

impl Comparator for MinCmp {
    fn better(new: f64, existing: f64) -> bool {
        new <= existing
    }

    fn name() -> &'static str {
        "min"
    }
}
impl Comparator for MaxCmp {
    fn better(new: f64, existing: f64) -> bool {
        new >= existing
    }

    fn name() -> &'static str {
        "max"
    }
}

#[derive(Clone, Copy)]
pub struct LevelView {
    pub window_size: u64,
    pub best_idx: Option<usize>,
}

pub struct SharedMonotonicQueue<C: Comparator> {
    pub entries: VecDeque<(u64, f64)>,
    pub views: [LevelView; 8],
    _cmp: std::marker::PhantomData<C>,
}

impl<C: Comparator> SharedMonotonicQueue<C> {
    pub fn new(window_sizes: [u64; 8]) -> Self {
        Self {
            entries: VecDeque::new(),
            views: std::array::from_fn(|i| LevelView {
                window_size: window_sizes[i],
                best_idx: None,
            }),
            _cmp: std::marker::PhantomData,
        }
    }

    pub fn push(&mut self, index: u64, value: f64) {
        // Enforce monotonic invariant: remove worse values from the back
        while let Some(&(_, back_val)) = self.entries.back() {
            if C::better(value, back_val) {
                self.entries.pop_back();

                for view in self.views.iter_mut() {
                    if let Some(i) = view.best_idx {
                        if i == self.entries.len() {
                            // We're removing current best — set new one to upcoming push
                            view.best_idx = Some(self.entries.len());
                        } else if i > self.entries.len() {
                            view.best_idx = None;
                        }
                    }
                }
            } else {
                break;
            }
        }

        // tracing::info!(
        //     "{}, pushing value: ({index}, {value}) @ {}",
        //     C::name(),
        //     self.entries.len()
        // );
        self.entries.push_back((index, value));

        for view in self.views.iter_mut() {
            let min_index = (index + 1).saturating_sub(view.window_size);
            match view.best_idx {
                Some(i) => {
                    let (idx, best_val) = self.entries[i];
                    if idx < min_index {
                        // tracing::info!(
                        //     "{}: pushing best_idx {} for level {} to one back ({idx} < {min_index})",
                        //     C::name(),
                        //     view.best_idx.unwrap(),
                        //     view.window_size,
                        // );
                        view.best_idx = view.best_idx.map(|i| i + 1);
                    }
                    if C::better(value, best_val) {
                        // tracing::info!(
                        //     "{}: moving best_idx {} for level {} to {}",
                        //     C::name(),
                        //     view.best_idx.unwrap(),
                        //     view.window_size,
                        //     self.entries.len()
                        // );
                        view.best_idx = Some(self.entries.len() - 1);
                    }
                }
                None => {
                    view.best_idx = Some(self.entries.len() - 1);
                }
            }
        }
    }

    pub fn evict_older_than(&mut self, current_index: u64) {
        let max_window = self.views.iter().map(|v| v.window_size).max().unwrap_or(0);
        let oldest_allowed = current_index.saturating_sub(max_window);

        while let Some(&(idx, _)) = self.entries.front() {
            if idx < oldest_allowed {
                self.entries.pop_front();

                for view in self.views.iter_mut() {
                    if let Some(i) = view.best_idx {
                        if i == 0 {
                            view.best_idx = None;
                        } else {
                            view.best_idx = Some(i - 1);
                        }
                    }
                }
            } else {
                break;
            }
        }
    }

    // pub fn refresh_best(&mut self, level: usize, current_index: u64) {
    //     let view = &mut self.views[level];
    //     let min_index = current_index.saturating_sub(view.window_size);
    //
    //     let mut best: Option<(usize, f64)> = None;
    //
    //     for (i, &(idx, val)) in self.entries.iter().enumerate() {
    //         if idx < min_index {
    //             continue;
    //         }
    //         match best {
    //             Some((_, best_val)) if C::better(val, best_val) => best = Some((i, val)),
    //             None => best = Some((i, val)),
    //             _ => {}
    //         }
    //     }
    //
    //     view.best_idx = best.map(|(i, _)| i);
    // }

    pub fn best(&self, level: usize) -> Option<f64> {
        // tracing::info!("smq {}: {:?}", C::name(), self.entries);
        self.views[level]
            .best_idx
            .and_then(|i| self.entries.get(i).map(|&(_, v)| v))
    }

    pub fn debug_best_indexes(&self) -> [Option<usize>; 8] {
        std::array::from_fn(|i| self.views[i].best_idx)
    }
}
