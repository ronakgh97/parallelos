**Parallelos** is a task scheduler for `massively parallel computing`.
The implementation & API is **very experimental (edge/deadlock cases are documented)** and combined ideas of for what I
found from this [wiki page](https://en.wikipedia.org/wiki/Category:Scheduling_algorithms)

Currently, the scheduler more or less implements `list scheduling` algorithm with `longest-processing-time-first` (LPT)
heuristic.

> Nested parallelism is not implemented, anything weird or funny can lead to deadlocks 💀.

Benchmark:

```terminaloutput
# run via cargo bench --bench bench

warming up with 1042 tasks
running 4168 tasks with 16 parallelos workers (payload 34 MB)
time taken to submit tasks: 0.0032066s

worker   tasks        busy     util
------------------------------------
    0     213      104.91s    91.89%
    1     285      109.47s    95.89%
    2     248      105.58s    92.48%
    3     249      105.73s    92.61%
    4     273      114.00s    99.86%
    5     259      108.18s    94.75%
    6     258      107.64s    94.28%
    7     287      107.14s    93.85%
    8     260      110.36s    96.66%
    9     294      109.27s    95.71%
   10     248      107.93s    94.54%
   11     317      110.95s    97.18%
   12     213      106.85s    93.59%
   13     248      105.06s    92.03%
   14     242      107.13s    93.83%
   15     274      108.01s    94.60%
------------------------------------
hex: 160.416 sec (9.28%) 
sha: 155.527 sec (9.00%) 
aes: 38.764 sec (2.24%) 
zstd: 1373.481 sec (79.47%)

parallelos
achieved makespan: 114.169 sec
ideal makespan: 108.014 sec
total busy: 1728.225 sec
total util: 94.61%

running rayon for comparison with same config
hex: 177.392 sec (9.77%) 
sha: 147.894 sec (8.15%) 
aes: 38.991 sec (2.15%) 
zstd: 1451.092 sec (79.93%)

rayon
achieved makespan: 113.798 sec
ideal makespan: 113.461 sec
total busy: 1815.369 sec
total util: 99.70%

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
