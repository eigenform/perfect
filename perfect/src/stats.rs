
use std::collections::*;
use std::io::Write;
use itertools::*;

use crate::harness::*;
use crate::events::*;

use csv;

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
        let (val, cnt) = self.histogram().into_iter()
            .max_by(|x,y| x.1.cmp(&y.1))
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

/// A list of observed values (represented with [`usize`]). 
///
/// NOTE: In general, each entry in the list is a *difference* between two 
/// values (returned by RDPMC or some other instruction providing a counter). 
#[derive(Clone)]
pub struct RawResults(pub Vec<usize>);
impl RawResults {
    /// Produce a set of "normalized" values with the given floor value. 
    pub fn normalize(&self, floor_min: i64) -> NormalizedResults {
        NormalizedResults(
            self.0.iter().map(|x| *x as i64 - floor_min as i64).collect()
        )
    }

    /// Return the [`f32`] average over values in this list. 
    pub fn get_avg(&self) -> f32 { 
        let sum = self.0.iter().sum::<usize>() as f32;
        sum / self.0.len() as f32
    }

    /// Return the [`f32`] variance over values in this list. 
    pub fn variance(&self) -> f32 { 
        let len = self.0.len() as f32;
        let mean: f32 = (self.0.iter().sum::<usize>() as f32) / len;
        let sum = self.0.iter()
            .map(|x| (*x as f32 - mean) * (*x as f32 - mean))
            .fold(0.0, |res, x| res + x);
        sum / (len - 1.0)
    }

    /// Return the [`f32`] standard deviation over values in this list. 
    pub fn stddev(&self) -> f32 { 
        self.variance().sqrt()
    }
}

/// A *normalized* list of observed values (represented with [`i64`]).
#[derive(Clone)]
pub struct NormalizedResults(pub Vec<i64>);

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

/// Implemented on suitable "input" variable types associated with a particular 
/// set of measurements (ie. in [`EventResults`]). 
///
/// NOTE: So far, we really only expect this to be [`usize`].
pub trait DependentVariable: 
    Copy + Clone + Default + PartialEq + Eq + std::fmt::Display {}

impl <T: Copy + Clone + Default + PartialEq + Eq + std::fmt::Display> 
DependentVariable for T {}

/// Set of results associated with a particular PMC event, where `I` is a type 
/// representing some variable associated with this set of observations 
/// (for instance, a variable number of emitted instructions). 
#[derive(Clone)]
pub struct EventResults<I: DependentVariable> {
    /// The event used to produce this data
    pub event: EventDesc,
    /// Some variable input value associated with each set of observations
    pub inputs: Vec<I>,
    /// Sets of observations
    pub data: Vec<RawResults>,
}
impl <I: DependentVariable> EventResults<I> {

    /// Create an empty set of observations for the given event `E`
    pub fn new<E: AsEventDesc>(event: E) -> Self { 
        Self { 
            event: event.as_desc(),
            data: Vec::new(),
            inputs: Vec::new(),
        }
    }

    /// Return the total number of observations in this set. 
    pub fn len(&self) -> usize { 
        assert!(self.inputs.len() == self.data.len());
        self.data.len()
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

    /// Return a list of [`f32`] averages across all results. 
    pub fn local_avg_f32(&self) -> Vec<f32> {
        self.data.iter().map(|d| {
            let len = d.len();
            d.iter().sum::<usize>() as f32 / len as f32
        }).collect()
    }

    /// Return a list of [integer] averages across all results. 
    pub fn local_avg_usize(&self) -> Vec<usize> {
        self.data.iter().map(|d| {
            let len = d.len();
            d.iter().sum::<usize>() / len
        }).collect()
    }

    /// Return a list of the minimum observed values across all results. 
    pub fn local_min(&self) -> Vec<usize> {
        self.data.iter().map(|d| d.get_min()).collect()
    }

    /// Return a list of the maximum observed values across all results. 
    pub fn local_max(&self) -> Vec<usize> {
        self.data.iter().map(|d| d.get_max()).collect()
    }

    /// Return a list of the mode for observed values across all results. 
    pub fn local_mode(&self) -> Vec<usize> {
        self.data.iter().map(|d| d.get_mode()).collect()
    }

    /// Return a list of the minimum/maximum observed values across all results. 
    pub fn local_minmax(&self) -> Vec<(usize, usize)> {
        std::iter::zip(self.local_min(), self.local_max()).collect()
    }

    /// Return the input value and result value of the first non-zero result 
    /// value that occured in *any* of the observations within this set. 
    pub fn local_min_first_nonzero(&self) -> Option<(usize, I)> { 
        let local_mins: Vec<usize> = self.local_min();
        let nz = local_mins.iter().enumerate().find(|(i,m)| **m != 0);
        if let Some((idx, min)) = nz { 
            return Some((*min, self.inputs[idx]));
        }
        return None;
    }

    // Obtain the minimum for each observation, then return a list of pairwise 
    // differences [as signed integers] between them. 
    pub fn local_min_pairwise_diff(&self) -> Vec<isize> {
        let local_mins: Vec<usize> = self.local_min();
        let mut diffs = Vec::new();
        for (x, y) in local_mins.iter().tuple_windows() {
            diffs.push((*y as isize) - (*x as isize));
        }
        diffs
    }
}

/// A set of related measurements taken with different PMC events, where 
/// `E` is the type of event (implementing [`AsEventDesc`]), and `I` is 
/// the type of dependent variable associated with each observation.
#[derive(Clone)]
pub struct ExperimentCaseResults<E: AsEventDesc, I: DependentVariable> {
    /// A string describing this case
    pub desc: &'static str,
    /// Results for each PMC event 
    pub data: BTreeMap<E, EventResults<I>>,
}
impl <E: AsEventDesc, I: DependentVariable> ExperimentCaseResults<E, I> {
    pub fn new(desc: &'static str) -> Self { 
        Self { desc, data: BTreeMap::new(), }
    }

    pub fn fs_name(&self) -> String {
        self.desc.replace(" ", "_").to_lowercase()
    }

    /// Record a set of results along with the associated input data and the 
    /// particular PMC event. 
    ///
    /// FIXME: This doesn't validate that the number of observations in 
    /// each set are the same (everything is just a `Vec`)
    pub fn record(&mut self, event: E, input: I, data: RawResults) {
        if let Some(mut records) = self.data.get_mut(&event) {
            records.push(data, input);
        } else { 
            let mut res = EventResults::new(event);
            res.push(data, input);
            self.data.insert(event, res);
        }
    }

    /// Return an iterator over the `i`-th set of observations for each event. 
    pub fn iter_ith_results(&self, i: usize) -> impl Iterator<Item = &RawResults> { 
        self.data.iter().map(move |(e, results)| { &results.data[i] })
    }

    /// Return an iterator over the `i`-th input variable for each event. 
    pub fn iter_ith_input(&self, i: usize) -> impl Iterator<Item = &I> { 
        self.data.iter().map(move |(e, results)| { &results.inputs[i] })
    }

    /// Write this set of results to a CSV file. 
    ///
    /// FIXME: This code is very slow and bad. Would probably be better if 
    /// this structure was reorganized.
    ///
    /// FIXME: For now, this simply takes the *mode* from each set of 
    /// observations. 
    ///
    pub fn write_csv(&self, filename: &str) { 
        let mut w = csv::Writer::from_path(filename).unwrap();

        // The first row is a set of labels for each column
        let mut fields = Vec::new();
        fields.push("input".to_string());
        for evt in self.data.keys() { 
            fields.push(evt.as_desc().name().to_string());
        }


        // FIXME: This doesn't validate that all results contain the 
        // same number of observations 
        let first = self.data.iter().next().unwrap();
        let mut num_samples = first.1.len();
        let num_columns = self.data.keys().len();

        let mut rows: Vec<(I, Vec<usize>)> = {
            vec![(I::default(), Vec::new()); num_samples]
        };

        // Fill out the dependent variable column. 
        // FIXME: We only look at one event, but there's no guarantee that 
        // these are the same across all recorded events. 
        for idx in 0..num_samples { 
            rows[idx].0 = first.1.inputs[idx];
        }

        // Fill out the column for each event
        for (evt, results) in self.data.iter() { 
            for idx in 0..num_samples { 
                rows[idx].1 = self.iter_ith_results(idx)
                    .map(|x| x.get_mode())
                    .collect();
            }
        }

        // Write all of the rows
        w.write_record(fields).unwrap();
        for row in rows { 
            let mut r = Vec::new();
            r.push(format!("{}", row.0));
            r.extend(row.1.iter().map(|x| format!("{}", x)));
            w.write_record(r).unwrap();
        }

    }
}

/// Set of results associated with a particular experiment. 
pub struct ExperimentResults<E: AsEventDesc, I: DependentVariable> { 
    pub data: Vec<ExperimentCaseResults<E, I>>,
}
impl <E: AsEventDesc, I: DependentVariable> ExperimentResults<E, I> {
    pub fn new() -> Self { 
        Self { data: Vec::new() }
    }
    pub fn push(&mut self, results: ExperimentCaseResults<E, I>) {
        self.data.push(results);
    }
}

impl <E: AsEventDesc, I: DependentVariable> ExperimentResults<E, I> {
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
}
