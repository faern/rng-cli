# rng - A random number generator CLI tool.

Contains a number of (pseudo) random number generator algorithms. Given one of these it writes an
infinite stream of bytes generated from that algorithm to stdout.

In unix terms it can be viewed as the equivalent of `cat /dev/urandom` but with a
selection of different (mostly user-space) PRNG algorithms to choose from. This is usually way
faster than `/dev/urandom` but can also provide lower entropy output, depending on which algorithm
is chosen.

This tool is more or less a CLI frontend for the awesome `rand` crate.

# How to use

```
rng [--seed <seed>] [--max threads] [--verbose] [<algorithm>]
```

If no arguments are given it uses the default algorithm and seeds it from the operating system.
You might want to change the algorithm in order to fit your needs. Maybe you need a faster algorithm
that is not cryptographically secure for example.

The `--seed <seed>` argument initializes the random number algorithm with the given `<seed>` instead
of obtaining some entropy from the operating system. You don't want to use this for any
cryptographical purposes. Giving a seed can be useful when you need determinism and must be able
to produce identical data over multiple runs.

The `--max-threads` argument sets an upper limit on how many threads the tool can use in
multithreaded mode. By default this is set to the number of hardware threads available on
the system. The exception is when `--seed` is specified or the algorithm is "os", then the
tool always runs in single-threaded mode.

## Example

We try using the PCG algoritm a few times. Here we see that without a seed it produces different
output each time, and with a seed it produces the same data as long as the seed is the same.

```bash
$ rng pcg | dd count=1024 | shasum
ae148d8b54ee544a50833c2a6915b0fca4cb95ed -
$ rng pcg | dd count=1024 | shasum
65a5651a2b1201adbe8bf60d9f7a2b940f6e188b -
$ rng pcg --seed 6 | dd count=1024 | shasum
fc14baccbc847339408457335dc67bc4c9785185 -
$ rng pcg --seed 6 | dd count=1024 | shasum
fc14baccbc847339408457335dc67bc4c9785185 -
$ rng pcg --seed 7 | dd count=1024 | shasum
c7a169d195c395867396d16d70c1a3c19f63b5cc -
```

# Why?

This tool was invented because I needed to benchmark IO (both filesystem and network) on Linux.
Reading from `/dev/urandom` was too slow, the machine spent too much time computing the actual data
and too little time performing the IO I wanted to test. On the other hand, getting data from
`/dev/zero` is fast! But any compression (common in both network protocols and filesystems) will be
able to optimize this stream down to almost nothing. And again, the actual IO is not being properly
benchmarked.

So what I needed was a very fast stream of incompressible data. The `rand` crate provides a unified
interface to a number of different random number algorithms. Some of them suiting my use case.
