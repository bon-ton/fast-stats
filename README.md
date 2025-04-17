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
    - `O(n)` space complexity, with small constant (~`2`)
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

Criterion generates a full HTML report you can open in a browser:

```zsh
open target/criterion/report/index.html   # macOS
```

```bash
xdg-open target/criterion/report/index.html  # Linux
```

#### Benchmark results on M2 chip

```txt
add_batch_100           time:   [5.9548 µs 6.4105 µs 7.1221 µs]
add_batch_1k            time:   [71.809 µs 75.399 µs 79.709 µs]
add_batch_10k           time:   [676.22 µs 712.48 µs 754.33 µs]

get_stats_k=4           time:   [10.635 ns 10.690 ns 10.758 ns]
get_stats_k=8           time:   [9.6433 ns 10.259 ns 11.086 ns]

add_and_get_stats_k=4   time:   [1.0651 ms 1.1112 ms 1.1589 ms]
add_and_get_stats_k=8   time:   [774.93 µs 813.75 µs 860.32 µs]

POST /add_batch/1k      time:   [130.86 µs 141.78 µs 153.65 µs]
POST /add_batch/10k     time:   [1.3755 ms 1.4271 ms 1.4882 ms]

GET /stats?k=7          time:   [1.3151 µs 1.3426 µs 1.3744 µs]
GET /stats?k=8          time:   [1.3048 µs 1.3525 µs 1.4106 µs]
```