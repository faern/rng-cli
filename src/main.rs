use rand::{RngCore, SeedableRng};
use std::{
    fmt,
    io::{self, Write},
};
use structopt::StructOpt;

/// The number of bytes to handle in each generate-print iteration.
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

        Contains a number of (pseudo) random number generator algorithms. Given one of these it
        prints an infinite stream of bytes generated from that algorithm to stdout.
    ",
    rename_all = "kebab-case"
)]
struct Opt {
    /// Which random number generator algorithm to use. All of these are user-space PRNGs, except
    /// the "os" one. All user-space generators are seeded with entropy from the operating system,
    /// unless the --seed argument is given.
    /// Possible values are:
    ///
    /// * default - The default user-space PRNG. Considered cryptographically secure.
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

    /// Optionally seed the random number generator algorithm with a given 64 bit unsigned integer.
    /// This makes the output of the program identical for each run with the same algorithm and
    /// same seed.
    /// If this argument is not given, the PRNG will be seeded from the operating system.
    /// Specifying a seed is NOT recommended for cryptographic use.
    #[structopt(long)]
    seed: Option<u64>,
}

#[derive(Debug)]
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
            "default" => Ok(Algorithm::Default),
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

/// Initializes a given PRNG. Either from entropy provided by the OS, or from the given u64 seed.
macro_rules! init_rng {
    ($rng:ty, $seed:expr) => {
        match $seed {
            None => <$rng>::from_entropy(),
            Some(seed) => <$rng>::seed_from_u64(seed),
        }
    };
}

fn main() {
    let opt = Opt::from_args();
    let seed = opt.seed;

    match opt.algorithm.unwrap_or(Algorithm::Default) {
        Algorithm::Default => generate(init_rng!(rand::rngs::StdRng, seed)),
        Algorithm::Hc => generate(init_rng!(rand_hc::Hc128Rng, seed)),
        Algorithm::ChaCha8 => generate(init_rng!(rand_chacha::ChaCha8Rng, seed)),
        Algorithm::ChaCha12 => generate(init_rng!(rand_chacha::ChaCha12Rng, seed)),
        Algorithm::ChaCha20 => generate(init_rng!(rand_chacha::ChaCha20Rng, seed)),
        Algorithm::XorShift => generate(init_rng!(rand_xorshift::XorShiftRng, seed)),
        Algorithm::Pcg => generate(init_rng!(PcgRng, seed)),
        Algorithm::Os => {
            if seed.is_some() {
                eprintln!("WARNING: --seed is ignored when used with the OS PRNG");
            }
            generate(rand::rngs::OsRng);
        }
    }
}

/// Given a random number generator, prints the output of it to stdout forever, or until there
/// is an error printing to stdout. Usually because the pipe has closed.
fn generate(mut rng: impl RngCore) {
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    let mut buf = [0u8; BUFFER_SIZE];
    loop {
        rng.fill_bytes(&mut buf);
        if stdout_lock.write_all(&buf).is_err() {
            break;
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

impl std::error::Error for ParseAlgorithmError {}
