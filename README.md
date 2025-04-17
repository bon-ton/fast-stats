## Fast Stats

A high-performance, in-memory Rust microservice for computing real-time trading statistics — built for
single node processing.

---

### Features

- Real-time statistics with sliding window analysis
- `O(n)` add batch endpoint
- `O(1)` or worst-case `O(log n)` performance,
  for `n` number of strictly monotonic sub-sequence
- Fully in-memory — fast, no persistent storage
    - `O(n)` space complexity, with small constant (~`3`)
- Numerical stability
  with [Kahan–Babuška algorithm improved by Neumaier](https://en.wikipedia.org/wiki/Kahan_summation_algorithm)
    - Values up to `1e153` are supported, larger are skipped (ignored)
- 🧵Lock-free concurrent access across symbols using `DashMap`
- 🔒 No concurrent access within the same symbol, as per spec

---

### API Endpoints

* `POST /add_batch/`
  add a batch of `f64` values.
* `GET /stats/?symbol=AB&k=3`
  get stats over the most recent `10^k` values, for `1 ≤ k ≤ 8`.

### ⚙️ How It Works

* Each symbol has a dedicated `SymbolAggregator` (mutex-protected, stored in `DashMap`)
* Each aggregator maintains:
    * Shared circular buffer of values (`Vec<f64>`)
    * Multi-resolution stats for levels 1 to 8
    * Two shared monotonic queues for efficient `min`/`max` tracking
        * each has single ring of values (`VecDeque<u64, f64>`)
        * first element is the absolute index of value, which never resets
        * `64` bits should be enough to handle `10^5` add_batch reqs/s for few hundred years
* Stats use constant or logarithmic algorithms:
* `avg`/`var`: Kahan summation, updated on-line while adding, so `O(1)` stats
* `min`/`max`: Shared monotonic queues
    * `O(1)` stats for highest level `8`
    * `O(log n)` stats for lower levels
        * with lazy binary-search refresh and cache
        * good amortised trade-off, see code comments for rationale

### 🚀 Run the server

```bash
cargo run --release
```

Starts on http://localhost:3000.

### 🧪 Run all tests

```bash
cargo test --all-features
```

### 🧪 Run benchmarks

```bash
cargo bench --all-features
```

Includes correctness, eviction, and performance-related test coverage.