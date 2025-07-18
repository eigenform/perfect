//! Module with types for handling input arguments to measured code.

use rand::rngs::ThreadRng;

/// Auto-implemented on function types that are suitable for generating
/// input to a measured function.
///
/// [PerfectHarness::measure_vary] expects a type like this for varying the
/// inputs on each iteration of a test. Returns a tuple `(usize, usize)` with
/// values passed to the measured function via RDI and RSI.
///
/// The arguments to this function are:
///
/// - A mutable reference to the harness' [`ThreadRng`]
/// - The current iteration/test index for the associated input
///
pub trait InputGenerator:
    Fn(&mut ThreadRng, usize) -> (usize, usize) {}
impl <F: Fn(&mut ThreadRng, usize) -> (usize, usize)>
    InputGenerator for F {}

/// Strategy used by [PerfectHarness] to compute the set of inputs to the
/// measured function across all test runs.
#[derive(Clone)]
pub enum InputMethod<'a> {
    /// Fix the value of both arguments (RDI and RSI) across all test runs.
    Fixed(usize, usize),

    /// Provide a function/closure which computes the arguments (RDI and RSI)
    /// by using:
    /// - A mutable reference to the [`ThreadRng`] owned by the harness
    /// - The index of the current test run
    Random(&'static dyn Fn(&mut ThreadRng, usize) -> (usize, usize)),

    /// Provide a precomputed list of arguments (RDI and RSI).
    List(&'a Vec<(usize, usize)>),
}

