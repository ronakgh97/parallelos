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
time taken to submit tasks: 0.0047682s

worker   tasks        busy     util
------------------------------------
    0     303      107.33s    99.80%
    1     263      107.54s   100.00%
    2     267      105.60s    98.19%
    3     285      107.03s    99.52%
    4     210      105.32s    97.93%
    5     266      107.24s    99.72%
    6     256      107.01s    99.50%
    7     267      106.56s    99.09%
    8     247      106.40s    98.94%
    9     255      106.73s    99.24%
   10     301      107.35s    99.81%
   11     231      106.48s    99.00%
   12     277      106.59s    99.11%
   13     256      107.44s    99.90%
   14     253      104.43s    97.10%
   15     231      106.06s    98.62%
------------------------------------
hex: 168.674 sec (9.89%)
sha: 22.636 sec (1.33%)
aes: 39.230 sec (2.30%)
zstd: 1474.541 sec (86.48%)

achieved makespan: 107.547 sec
ideal makespan: 106.570 sec
total busy: 1705.114 sec
total util: 99.09%

# last updated: 11-06-2026
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
