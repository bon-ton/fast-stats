//! this module is superseded by shared_monotonic_queue
use crate::shared_monotonic_queue::Comparator;
use std::collections::VecDeque;
use std::marker::PhantomData;

pub struct MonotonicQueue<C: Comparator> {
    deque: VecDeque<(u64, f64)>,
    phantom_data: PhantomData<C>,
}

impl<C: Comparator> MonotonicQueue<C> {
    pub fn new() -> Self {
        Self {
            deque: VecDeque::new(),
            phantom_data: Default::default(),
        }
    }

    pub fn push(&mut self, index: u64, value: f64) {
        while let Some(&(_, back)) = self.deque.back() {
            if C::better(value, back) {
                self.deque.pop_back();
            } else {
                break;
            }
        }
        self.deque.push_back((index, value));
    }

    pub fn evict_older_than(&mut self, min_index: u64) {
        while let Some(&(idx, _old_best)) = self.deque.front() {
            if idx < min_index {
                self.deque.pop_front();
                // tracing::info!(
                //     "{}: evicting old best: {old_best} @ {idx} < {min_index}",
                //     C::name()
                // );
            } else {
                break;
            }
        }
    }

    pub fn best(&self) -> Option<f64> {
        tracing::info!("mq {}: {:?}", C::name(), self.deque);
        self.deque.front().map(|&(_, v)| v)
    }
}
