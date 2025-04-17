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

Includes correctness, eviction.

```bash
cargo test --all-features
```

### 🧪 Run benchmarks

```bash
cargo bench --all-features
```

#### Benchmark results on M2 chip

```txt
add_batch_10k           time:   [429.63 µs 439.18 µs 450.89 µs]
Found 10 outliers among 100 measurements (10.00%)
  7 (7.00%) high mild
  3 (3.00%) high severe

get_stats_k=4           time:   [6.9251 ns 7.0494 ns 7.2196 ns]
Found 7 outliers among 100 measurements (7.00%)
  5 (5.00%) high mild
  2 (2.00%) high severe

get_stats_k=8           time:   [5.6252 ns 5.6837 ns 5.7524 ns]
Found 9 outliers among 100 measurements (9.00%)
  5 (5.00%) high mild
  4 (4.00%) high severe

add_and_get_stats_k=4   time:   [1.0665 ms 1.2443 ms 1.4579 ms]
Found 6 outliers among 100 measurements (6.00%)
  6 (6.00%) high mild

add_and_get_stats_k=8   time:   [632.19 µs 667.88 µs 709.44 µs]
Found 14 outliers among 100 measurements (14.00%)
  9 (9.00%) high mild
  5 (5.00%) high severe

POST /add_batch         time:   [915.53 µs 925.08 µs 935.03 µs]
Found 12 outliers among 100 measurements (12.00%)
  9 (9.00%) high mild
  3 (3.00%) high severe

GET /stats?k=7          time:   [1.2768 µs 1.2890 µs 1.3002 µs]
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

GET /stats?k=8          time:   [1.2521 µs 1.2558 µs 1.2605 µs]
Found 7 outliers among 100 measurements (7.00%)
  4 (4.00%) high mild
  3 (3.00%) high severe
```