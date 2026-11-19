**Parallelos** is a task scheduler for `massively parallel computing`.
The implementation & API is **very experimental (edge/deadlock cases are documented)** and combined ideas of for what I
found from this [wiki page](https://en.wikipedia.org/wiki/Category:Scheduling_algorithms)

Currently, the scheduler more or less implements `list scheduling` algorithm with `longest-processing-time-first` (LPT)
heuristic.

> Nested parallelism is not implemented, anything weird or funny can lead to deadlocks 💀.

Benchmark:

```terminaloutput
# run via cargo bench --bench bench

warming up with 1536 tasks
running 4168 tasks with 16 workers (payload 34 MB)
time taken to submit tasks: 0.0092483s

worker   tasks        busy     util
------------------------------------
    0     254      117.84s   100.00%
    1     215      113.70s    96.49%
    2     274      115.57s    98.08%
    3     278      115.54s    98.05%
    4     280      115.80s    98.27%
    5     263      115.82s    98.29%
    6     284      113.30s    96.14%
    7     262      114.19s    96.91%
    8     282      117.09s    99.37%
    9     310      116.30s    98.69%
   10     291      116.28s    98.68%
   11     223      115.42s    97.95%
   12     229      115.64s    98.13%
   13     241      115.87s    98.33%
   14     214      113.96s    96.71%
   15     268      115.12s    97.69%
------------------------------------
hex: 180.853 sec (9.79%) 
sha: 23.803 sec (1.29%) 
aes: 39.367 sec (2.13%) 
zstd: 1603.396 sec (86.79%)

achieved makespan: 117.839 sec
ideal makespan: 115.466 sec
total busy: 1847.449 sec
total util: 97.99%

# last updated: 11-19-2026
```

References:

- https://en.wikipedia.org/wiki/List_scheduling
- https://en.wikipedia.org/wiki/Longest-processing-time-first_scheduling
- https://en.wikipedia.org/wiki/Optimal_job_scheduling
- https://en.wikipedia.org/wiki/Identical-machines_scheduling
- https://en.wikipedia.org/wiki/Uniform-machines_scheduling
- https://en.wikipedia.org/wiki/Shortest_job_next
- https://en.wikipedia.org/wiki/Shortest_remaining_time
- https://en.wikipedia.org/wiki/Highest_response_ratio_next
- https://en.wikipedia.org/wiki/Heterogeneous_earliest_finish_time
