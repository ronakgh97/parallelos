**Parallelos** is a task scheduler for `massively parallel computing`.
The implementation & API is **very experimental (edge/deadlock cases are documented)** and combined ideas of for what I
found from this [wiki page](https://en.wikipedia.org/wiki/Category:Scheduling_algorithms)

Currently, the scheduler more or less implements `list scheduling` algorithm with `longest-processing-time-first` (LPT)
heuristic.

> Nested parallelism is not implemented, anything weird or funny can lead to deadlocks 💀.

Benchmark:

```terminaloutput
# run via cargo bench --bench bench

warming up with 2234 tasks
running 4468 tasks with 16 parallelos workers (payload 34 MB)
time taken to submit tasks: 0.0174415s

worker   tasks        busy     util
------------------------------------
    0     302      112.59s    98.66%
    1     264      111.59s    97.79%
    2     267      111.91s    98.06%
    3     288      111.44s    97.65%
    4     277      111.09s    97.35%
    5     309      112.75s    98.80%
    6     307      113.13s    99.14%
    7     286      112.12s    98.25%
    8     262      113.79s    99.71%
    9     245      110.31s    96.66%
   10     285      111.93s    98.08%
   11     278      110.01s    96.40%
   12     277      110.56s    96.88%
   13     270      112.86s    98.89%
   14     266      111.25s    97.49%
   15     285      111.77s    97.94%
------------------------------------
hex: 179.272 sec (10.02%) 
sha: 114.583 sec (6.40%) 
aes: 38.736 sec (2.17%) 
zstd: 1456.472 sec (81.41%)

parallelos
achieved makespan: 114.119 sec
ideal makespan: 111.819 sec
total busy: 1789.101 sec
total util: 97.98%

running rayon for comparison with same config
hex: 194.109 sec (10.55%) 
sha: 107.243 sec (5.83%) 
aes: 38.363 sec (2.09%) 
zstd: 1500.054 sec (81.53%)

rayon
achieved makespan: 115.351 sec
ideal makespan: 114.986 sec
total busy: 1839.769 sec
total util: 99.68%

# last updated: 11-20-2026
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
