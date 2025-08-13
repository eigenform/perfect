
use std::collections::*;
use std::io::Write;
use itertools::*;

use crate::harness::*;
use crate::events::*;

/// A list of observed values. 
///
/// In general, each entry in the list is a *difference* between two values
/// (returned by RDPMC or some other instruction providing a counter). 
#[derive(Clone)]
pub struct RawResults(pub Vec<usize>);
impl RawResults {
    /// Produce a set of "normalized" values with the given floor value. 
    pub fn normalize(&self, floor_min: i64) -> NormalizedResults {
        NormalizedResults(
            self.0.iter().map(|x| *x as i64 - floor_min as i64).collect()
        )
    }
}

/// A *normalized* list of observed values. 
#[derive(Clone)]
pub struct NormalizedResults(pub Vec<i64>);

/// Implemented on types which contain a list of observed values. 
pub trait ResultList<D: Copy + Ord> { 
    /// Return a reference to the list of values.
    fn data(&self) -> &Vec<D>;

    fn get_value(&self, idx: usize) -> Option<D> {
        self.data().get(idx).copied()
    }
    fn get_last_value(&self) -> D {
        self.data().last().copied().unwrap()
    }


    /// Return the number of observed values.
    fn len(&self) -> usize { self.data().len() }

    /// Return the minimum value in the list.
    fn get_min(&self) -> D { *self.data().iter().min().unwrap() }

    /// Return the maximum value in the list.
    fn get_max(&self) -> D { *self.data().iter().max().unwrap() }

    /// Return the most-frequent value in the list.
    fn get_mode(&self) -> D { 
        let (val, cnt) = self.histogram().into_iter().max_by(|x,y| x.1.cmp(&y.1))
            .unwrap();
        val
    }

    /// Return an iterator over values in the list.
    fn iter<'a>(&'a self) -> impl Iterator<Item=&'a D> where D: 'a { 
        self.data().iter() 
    }

    /// Return a histogram counting the distribution of all values in the list.
    fn histogram(&self) -> BTreeMap<D, usize> {
        let mut dist = BTreeMap::new();
        for r in self.data().iter() {
            if let Some(cnt) = dist.get_mut(r) {
                *cnt += 1;
            } else {
                dist.insert(*r, 1);
            }
        }
        dist
    }

    /// Returns the number of times that a particular value occurs in the list.
    fn count(&self, val: D) -> usize { 
        self.iter().filter(|x| **x == val).count()
    }

    /// Return the indexes of all occurences of a particular value in the list.
    fn find(&self, val: D) -> Vec<usize> {
        self.iter().enumerate().filter(|(idx, x)| **x == val)
            .map(|(idx, x)| idx).collect()
    }

    /// Return the indexes of all values in the list for which the given 
    /// function `f` returns `true`. 
    fn filter(&self, mut f: impl FnMut(D) -> bool) -> Vec<usize> {
        self.iter().enumerate().filter(|(idx, x)| f(**x))
            .map(|(idx, x)| idx).collect()
    }
}

impl ResultList<usize> for RawResults {
    fn data(&self) -> &Vec<usize> { &self.0 }
}
impl ResultList<i64> for NormalizedResults {
    fn data(&self) -> &Vec<i64> { &self.0 }
}
impl ResultList<usize> for MeasureResults {
    fn data(&self) -> &Vec<usize> { &self.data.0 }
}

/// Results returned by [PerfectHarness::measure].
#[derive(Clone)]
pub struct MeasureResults {
    /// Set of observations from the performance counters
    pub data: RawResults,

    /// The PMC event associated with the result data
    pub event: EventDesc,

    /// Set of recorded [integer] GPR states across all test iterations
    pub gpr_dumps: Option<Vec<GprState>>,

    /// Set of recorded [vector] GPR states across all test iterations
    pub vgpr_dumps: Option<Vec<VectorGprState>>,

    /// Set of inputs (from RDI and RSI) across all test iterations
    pub inputs: Option<Vec<(usize, usize)>>,
}
impl MeasureResults {
    /// Return the PMC event ID associated with these results.
    pub fn event_id(&self) -> u16 { self.event.id() }
    /// Return the PMC event mask associated with these results.
    pub fn event_mask(&self) -> u8 { self.event.mask() }
}


/// Set of results associated with a particular PMC event, where `I` is a type 
/// representing some variable associated with this set of observations 
/// (for instance, a variable number of emitted instructions). 
#[derive(Clone)]
pub struct EventResults<I: Copy + Clone> {
    /// The event used to produce this data
    pub event: EventDesc,
    /// Some variable input value associated with each set of observations
    pub inputs: Vec<I>,
    /// Sets of observations
    pub data: Vec<RawResults>,
}
impl <I: Copy + Clone> EventResults<I> {

    /// Create an empty set of observations for the given event `E`
    pub fn new<E: AsEventDesc>(event: E) -> Self { 
        Self { 
            event: event.as_desc(),
            data: Vec::new(),
            inputs: Vec::new(),
        }
    }

    /// Add an observation to the set. 
    pub fn push(&mut self, data: RawResults, input: I) {
        self.data.push(data);
        self.inputs.push(input);
    }

    /// Return the maximum observed value and the associated input.
    pub fn global_max(&self) -> (usize, I) { 
        let mut global_max = usize::MIN;
        let mut input = self.inputs[0];
        for (idx, results) in self.data.iter().enumerate() {
            let local_max = results.iter().max().unwrap();
            if *local_max > global_max { 
                global_max = *local_max;
                input = self.inputs[idx];
            }
        }
        (global_max, input)
    }

    /// Return the minimum observed value and the associated input. 
    pub fn global_min(&self) -> (usize, I) { 
        let mut global_min = usize::MAX;
        let mut input = self.inputs[0];
        for (idx, results) in self.data.iter().enumerate() {
            let local_min = results.iter().max().unwrap();
            if *local_min < global_min { 
                global_min = *local_min;
                input = self.inputs[idx];
            }
        }
        (global_min, input)
    }

    pub fn local_avg_f32(&self) -> Vec<f32> {
        self.data.iter().map(|d| {
            let len = d.len();
            d.iter().sum::<usize>() as f32 / len as f32
        }).collect()
    }

    pub fn local_avg_usize(&self) -> Vec<usize> {
        self.data.iter().map(|d| {
            let len = d.len();
            d.iter().sum::<usize>() / len
        }).collect()
    }


    /// Return a list of the minimum observed values across all results. 
    pub fn local_min(&self) -> Vec<usize> {
        self.data.iter().map(|d| *d.iter().min().unwrap()).collect()
    }

    /// Return a list of the maximum observed values across all results. 
    pub fn local_max(&self) -> Vec<usize> {
        self.data.iter().map(|d| *d.iter().max().unwrap()).collect()
    }

    pub fn local_minmax(&self) -> Vec<(usize, usize)> {
        //self.data.iter().map(|d| *d.data.iter().min().unwrap())
        //    .zip(self.data.iter().map(|d| *d.data.iter().max().unwrap()))
        //    .collect()
        std::iter::zip(self.local_min(), self.local_max()).collect()
    }


    pub fn local_min_first_nonzero(&self) -> Option<(usize, I)> { 
        let local_mins: Vec<usize> = self.data.iter()
            .map(|d| *d.iter().min().unwrap())
            .collect();

        let nz = local_mins.iter().enumerate().find(|(i,m)| **m != 0);
        if let Some((idx, min)) = nz { 
            return Some((*min, self.inputs[idx]));
        }
        return None;
    }



    pub fn local_min_pairwise_diff(&self) -> Vec<isize> {

        let local_mins: Vec<usize> = self.data.iter()
            .map(|d| *d.iter().min().unwrap())
            .collect();

        let mut diffs = Vec::new();
        for (x, y) in local_mins.iter().tuple_windows() {
            diffs.push((*y as isize) - (*x as isize));
        }

        diffs
    }
}

/// Set of results associated with a particular case.
#[derive(Clone)]
pub struct ExperimentCaseResults<E: AsEventDesc, I: Copy + Clone> {
    /// A string describing this case
    pub desc: &'static str,
    /// Results for each PMC event 
    pub data: BTreeMap<E, EventResults<I>>,
}
impl <E: AsEventDesc, I: Copy + Clone> ExperimentCaseResults<E, I> {
    pub fn new(desc: &'static str) -> Self { 
        Self { 
            desc,
            data: BTreeMap::new(),
        }
    }
    pub fn fs_name(&self) -> String {
        self.desc.replace(" ", "_").to_lowercase()
    }

    /// Record a set of results along with the associated input data and the 
    /// particular PMC event. 
    pub fn record(&mut self, event: E, input: I, data: RawResults) {
        if let Some(mut records) = self.data.get_mut(&event) {
            records.push(data, input);
        } else { 
            let mut res = EventResults::new(event);
            res.push(data, input);
            self.data.insert(event, res);
        }
    }
}

/// Set of results associated with a particular experiment. 
pub struct ExperimentResults<E: AsEventDesc, I: Copy> { 
    pub data: Vec<ExperimentCaseResults<E, I>>,
}
impl <E: AsEventDesc, I: Copy> ExperimentResults<E, I> {
    pub fn new() -> Self { 
        Self { data: Vec::new() }
    }
    pub fn push(&mut self, results: ExperimentCaseResults<E, I>) {
        self.data.push(results);
    }
}

impl <E: AsEventDesc, I: Copy + std::fmt::Display> ExperimentResults<E, I> {
    fn generate_filename(
        case_results: &ExperimentCaseResults<E, I>,
        event: &E,
        event_results: &EventResults<I>
    ) -> String
    {
        let edesc = event.as_desc();
        let case_name = case_results.fs_name();
        format!("/tmp/{}__{:02x}_{:02x}_{}.dat", 
            case_results.fs_name(), 
            edesc.id(), 
            edesc.mask(), 
            edesc.fs_name()
        )
    }

    //pub fn write_results_freq(&self) {
    //    for case_results in self.data.iter() {
    //        for (event, event_results) in case_results.data.iter() {
    //            let path = Self::generate_filename(
    //                case_results, event, event_results
    //            );
    //            let mut f = std::fs::OpenOptions::new()
    //                .write(true).create(true).truncate(true)
    //                .open(&path).unwrap();

    //            println!("[*] Writing min/avg/max results to {}", path);
    //            writeln!(f, "# {}", event.as_desc().name()).unwrap();

    //            let one_counts = event_results.data.iter()
    //                .map(|results| results.count(1));

    //            let iterator = event_results.inputs.iter()
    //                .zip(one_counts);
    //            for ((input, cnt)) in iterator {
    //                writeln!(f, "input={} cnt={}",
    //                    input, cnt
    //                ).unwrap();
    //            }
    //        }
    //    }

    //}

    //// NOTE: Temporary hack for dumping results to disk
    //pub fn write_results(&self) {
    //    for case_results in self.data.iter() {
    //        for (event, event_results) in case_results.data.iter() {
    //            let path = Self::generate_filename(
    //                case_results, event, event_results
    //            );
    //            let mut f = std::fs::OpenOptions::new()
    //                .write(true).create(true).truncate(true)
    //                .open(&path).unwrap();

    //            println!("[*] Writing min/avg/max results to {}", path);
    //            writeln!(f, "# {}", event.as_desc().name()).unwrap();
    //            let minmax = event_results.local_minmax();
    //            let avgs = event_results.local_avg_usize();
    //            let iterator = event_results.inputs.iter()
    //                .zip(avgs.iter()).zip(minmax.iter());
    //            for ((input, avg), (min, max)) in iterator {
    //                writeln!(f, "input={} min={} avg={} max={}", 
    //                    input, min, avg, max
    //                ).unwrap();
    //            }
    //        }
    //    }
    //}
}
