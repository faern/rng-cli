use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;
use structopt::StructOpt;

mod formatting;
mod platform;

/// The number of bytes to handle in each generate-write iteration.
const BUFFER_SIZE: usize = 64 * 1024;

// We select PCG algorithm depending on platform. In order to get the best performance possible.
// This code is copied from the implementation of `SmallRng` in the `rand` crate.
// `SmallRng` does not guarantee it will always stick to PCG, otherwise we could use that wrapper
// directly.
#[cfg(all(not(target_os = "emscripten"), target_pointer_width = "64"))]
type PcgRng = rand_pcg::Pcg64Mcg;
#[cfg(not(all(not(target_os = "emscripten"), target_pointer_width = "64")))]
type PcgRng = rand_pcg::Pcg32;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "rng",
    author = "Linus Färnstrand <faern@faern.net>",
    about = "
        A random data generator CLI tool.

        Contains a number of (pseudo) random number generator (PRNG) algorithms.
        Given one of these it writes an infinite stream of bytes generated from
        that algorithm to stdout.

        By default this tool operates in a multi threaded mode where new worker threads are
        spawned until the write speed to stdout is saturated. This means multiple PRNG instances
        are executed in parallel and their generated data is interleaved to stdout. See
        --max-threads for more.
    ",
    rename_all = "kebab-case"
)]
struct Opt {
    /// The random number generator algorithm to use. All of these are user-space PRNGs, except
    /// "os". All user-space generators are seeded with entropy from the operating system,
    /// unless the --seed argument is given.
    ///
    /// If no algorithm is specified, a cryptographically secure algorithm with good performance
    /// is used.
    ///
    /// Possible values are:
    ///
    /// * hc - A cryptographically secure random number generator that uses the HC-128 algorithm.
    ///
    /// * chacha[8,12,20] - A cryptographically secure random number generator that uses the ChaCha
    /// algorithm. Uses 8, 12 or 20 rounds. Defaults to 20 rounds if the number is not specified.
    ///
    /// * xorshift - This algorithm is NOT suitable for cryptographic purposes, but is fast.
    ///
    /// * pcg - This algorithm is NOT considered cryptographically secure. But it has good
    /// statistical quality and is usually the fastest algorithm in this tool.
    ///
    /// * os - A random number generator that retrieves randomness from the operating system.
    /// Usually cryptograhically secure, but depends on the OS. Usually much slower than the
    /// user-space PRNGs. The --seed argument can't be used with this algorithm, as the operating
    /// system is in control of providing the data.
    algorithm: Option<Algorithm>,

    /// Seeds the random number generator algorithm with a given 64 bit unsigned integer.
    /// This makes the output of the program identical for each run with the same algorithm and
    /// same seed.
    /// If this argument is not given, the PRNG will be seeded from the operating system.
    /// Specifying a seed is NOT recommended for cryptographic use.
    ///
    /// Only single threaded operation is possible when a seed is specified.
    #[structopt(long)]
    seed: Option<u64>,

    /// Sets an upper limit on the number of worker threads to spawn for generating the random data.
    /// If not specified, the number of available hardware threads is used as the max number of
    /// worker threads.
    ///
    /// This is the *max* number of threads. It will always start with a single worker thread and
    /// only spawn new ones if generating the data is slower than writing it to stdout.
    /// So in general, slower algorithms will spawn more worker threads to saturate
    /// stdout writing speed.
    ///
    /// Specify --max-threads 1 to activate a special single threaded mode that is more efficient,
    /// but where output speed is limited by the performance of a single core.
    ///
    /// If a seed is specified, max threads will be ignored and the tool will work in single
    /// threaded mode. The same holds for the 'os' algorithm as no speed improvement is
    /// gained from trying to extract randomness from the OS in parallel.
    #[structopt(long, short = "t")]
    max_threads: Option<usize>,

    /// Activates verbose mode, where extra information will be printed to stderr.
    #[structopt(long, short)]
    verbose: bool,

    /// Writes to <output> instead of stdout.
    #[structopt(long, short)]
    output: Option<PathBuf>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Algorithm {
    Default,
    Hc,
    ChaCha8,
    ChaCha12,
    ChaCha20,
    XorShift,
    Pcg,
    Os,
}

impl std::str::FromStr for Algorithm {
    type Err = ParseAlgorithmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hc" => Ok(Algorithm::Hc),
            "chacha" | "chacha20" => Ok(Algorithm::ChaCha20),
            "chacha8" => Ok(Algorithm::ChaCha8),
            "chacha12" => Ok(Algorithm::ChaCha12),
            "xorshift" => Ok(Algorithm::XorShift),
            "pcg" => Ok(Algorithm::Pcg),
            "os" => Ok(Algorithm::Os),
            _ => Err(ParseAlgorithmError(())),
        }
    }
}

#[derive(Debug)]
struct ParseAlgorithmError(());

impl fmt::Display for ParseAlgorithmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Invalid algorithm. See --help for a list of valid options."
        )
    }
}

fn main() {
    let opt = Opt::from_args();
    let algorithm = opt.algorithm.unwrap_or(Algorithm::Default);
    let seed = opt.seed;

    let max_threads = if seed.is_some() || algorithm == Algorithm::Os {
        if opt.max_threads.is_some() && seed.is_some() {
            eprintln!(
                "WARNING: --max-threads is ignored when a seed is specified. \
                Manually seeded randomness generation must be single threaded."
            );
        }
        if opt.max_threads.is_some() && algorithm == Algorithm::Os {
            eprintln!("WARNING: --max-threads is ignored with the 'os' PRNG");
        }
        1
    } else {
        opt.max_threads.unwrap_or_else(num_cpus::get)
    };

    // Prepare the writer (stdout/file) to write all data to
    let stdout = io::stdout();
    let mut output = match opt.output {
        None => Output::Stdout(stdout.lock()),
        Some(path) => {
            let file = fs::File::create(&path).unwrap_or_else(|e| {
                eprintln!("Failed to open output file: {}", e);
                std::process::exit(1);
            });
            Output::File(file)
        }
    };

    let mut bytes_written: u64 = 0;
    let should_abort = platform::abort_handle();
    let write_fn = |buf: &[u8; BUFFER_SIZE]| {
        if output.write_all(&*buf).is_err() {
            return true;
        }
        bytes_written += crate::BUFFER_SIZE as u64;
        should_abort()
    };

    let start = Instant::now();
    // Start generating the data and writing it
    match max_threads {
        0 | 1 => singlethreaded::run(algorithm, seed, write_fn),
        max_threads => multithreaded::run(algorithm, max_threads, write_fn, opt.verbose),
    }

    // Print statistics about how much was written and in what time
    if opt.verbose {
        let elapsed_seconds = start.elapsed().as_millis() as f64 / 1000.0;
        let bytes_per_second = bytes_written as f64 / elapsed_seconds;
        eprintln!(
            "{} ({} bytes) written in {:.1} seconds = {}/s",
            formatting::format_bytes_written(bytes_written),
            bytes_written,
            elapsed_seconds,
            formatting::format_bytes_written(bytes_per_second as u64),
        );
    }
}

mod multithreaded {
    use super::Algorithm;
    use crossbeam_channel::{Receiver, Sender};
    use rand::{RngCore, SeedableRng};
    use std::thread;

    pub(crate) fn run<F: FnMut(&[u8; crate::BUFFER_SIZE]) -> bool>(
        algorithm: Algorithm,
        max_threads: usize,
        write_fn: F,
        verbose: bool,
    ) {
        let run_fn = match algorithm {
            Algorithm::Default => run_internal::<rand::rngs::StdRng, F>,
            Algorithm::Hc => run_internal::<rand_hc::Hc128Rng, F>,
            Algorithm::ChaCha8 => run_internal::<rand_chacha::ChaCha8Rng, F>,
            Algorithm::ChaCha12 => run_internal::<rand_chacha::ChaCha12Rng, F>,
            Algorithm::ChaCha20 => run_internal::<rand_chacha::ChaCha20Rng, F>,
            Algorithm::XorShift => run_internal::<rand_xorshift::XorShiftRng, F>,
            Algorithm::Pcg => run_internal::<crate::PcgRng, F>,
            Algorithm::Os => panic!("OS PRNG does not support multithreaded mode"),
        };
        run_fn(max_threads, verbose, write_fn);
    }

    fn run_internal<R: SeedableRng + RngCore, F: FnMut(&[u8; crate::BUFFER_SIZE]) -> bool>(
        max_threads: usize,
        verbose: bool,
        mut write_fn: F,
    ) {
        let (sender, receiver) = crossbeam_channel::bounded(max_threads);
        let (buf_return_sender, buf_return_receiver) =
            crossbeam_channel::bounded(max_threads.max(8));
        let mut threads = Vec::with_capacity(max_threads);
        loop {
            let buf = receiver.try_recv().unwrap_or_else(|_| {
                add_worker_thread::<R>(
                    &mut threads,
                    max_threads,
                    &sender,
                    &receiver,
                    &buf_return_receiver,
                    verbose,
                )
            });
            if write_fn(&*buf) {
                break;
            }
            let _ = buf_return_sender.try_send(buf);
        }
        drop(receiver);
        for thread in threads {
            thread.join().expect("Worker threads don't panic");
        }
    }

    /// Spawn another worker thread producing random data.
    /// This is cold since it will only happen a few times at the very start of the run.
    #[cold]
    #[inline(never)]
    fn add_worker_thread<R: SeedableRng + RngCore>(
        threads: &mut Vec<thread::JoinHandle<()>>,
        max_threads: usize,
        sender: &Sender<Box<[u8; crate::BUFFER_SIZE]>>,
        receiver: &Receiver<Box<[u8; crate::BUFFER_SIZE]>>,
        buf_return_receiver: &Receiver<Box<[u8; crate::BUFFER_SIZE]>>,
        verbose: bool,
    ) -> Box<[u8; crate::BUFFER_SIZE]> {
        if threads.len() < max_threads {
            let sender = sender.clone();
            let buf_return_receiver = buf_return_receiver.clone();
            threads.push(thread::spawn(move || {
                let mut rng = R::from_entropy();
                loop {
                    // Try to get a buffer from the writer thread, or allocate a new one
                    let mut buf = buf_return_receiver
                        .try_recv()
                        .unwrap_or_else(|_| Box::new([0u8; crate::BUFFER_SIZE]));
                    rng.fill_bytes(&mut *buf);
                    if sender.send(buf).is_err() {
                        break;
                    }
                }
            }));
            if verbose {
                eprintln!("Spawning worker thread {}", threads.len());
            }
        }
        receiver.recv().expect("The channel can't be closed here")
    }
}

mod singlethreaded {
    use crate::Algorithm;
    use rand::{RngCore, SeedableRng};

    pub(crate) fn run<F: FnMut(&[u8; crate::BUFFER_SIZE]) -> bool>(
        algorithm: Algorithm,
        seed: Option<u64>,
        write_fn: F,
    ) {
        let run_fn = match algorithm {
            Algorithm::Default => run_userspace::<rand::rngs::StdRng, F>,
            Algorithm::Hc => run_userspace::<rand_hc::Hc128Rng, F>,
            Algorithm::ChaCha8 => run_userspace::<rand_chacha::ChaCha8Rng, F>,
            Algorithm::ChaCha12 => run_userspace::<rand_chacha::ChaCha12Rng, F>,
            Algorithm::ChaCha20 => run_userspace::<rand_chacha::ChaCha20Rng, F>,
            Algorithm::XorShift => run_userspace::<rand_xorshift::XorShiftRng, F>,
            Algorithm::Pcg => run_userspace::<crate::PcgRng, F>,
            Algorithm::Os => run_os,
        };
        run_fn(seed, write_fn);
    }

    pub fn run_userspace<R: SeedableRng + RngCore, F: FnMut(&[u8; crate::BUFFER_SIZE]) -> bool>(
        seed: Option<u64>,
        write_fn: F,
    ) {
        let rng = match seed {
            None => R::from_entropy(),
            Some(seed) => R::seed_from_u64(seed),
        };
        generate_to_stdout(rng, write_fn)
    }

    fn run_os<F: FnMut(&[u8; crate::BUFFER_SIZE]) -> bool>(seed: Option<u64>, write_fn: F) {
        if seed.is_some() {
            eprintln!("WARNING: seed is ignored when used with the OS PRNG");
        }
        generate_to_stdout(rand::rngs::OsRng, write_fn)
    }

    /// Given a random number generator, writes the output of it to stdout forever, or until there
    /// is an error writing to stdout. Usually because the pipe has closed.
    fn generate_to_stdout<F: FnMut(&[u8; crate::BUFFER_SIZE]) -> bool>(
        mut rng: impl RngCore,
        mut write_fn: F,
    ) {
        let mut buf = [0u8; crate::BUFFER_SIZE];
        loop {
            rng.fill_bytes(&mut buf);
            if write_fn(&buf) {
                break;
            }
        }
    }
}

enum Output<'a> {
    Stdout(io::StdoutLock<'a>),
    File(fs::File),
}

impl<'a> Write for Output<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::Stdout(stdout) => stdout.write(buf),
            Output::File(f) => f.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::Stdout(stdout) => stdout.flush(),
            Output::File(f) => f.flush(),
        }
    }
}
