
use std::collections::*;
use std::io::Write;
use itertools::*;

use crate::harness::*;
use crate::events::*;

/// Set of results associated with a particular PMC event. 
#[derive(Clone)]
pub struct EventResults<I: Copy + Clone> {
    /// Input associated with each set of observations
    pub inputs: Vec<I>,
    /// Sets of observations
    pub data: Vec<MeasureResults>,
}
impl <I: Copy + Clone> EventResults<I> {
    pub fn new() -> Self { 
        Self { 
            data: Vec::new(),
            inputs: Vec::new(),
        }
    }

    pub fn push(&mut self, data: MeasureResults, input: I) {
        self.data.push(data);
        self.inputs.push(input);
    }

    /// Return the maximum observed value and the associated input.
    pub fn global_max(&self) -> (usize, I) { 
        let mut global_max = usize::MIN;
        let mut input = self.inputs[0];
        for (idx, results) in self.data.iter().enumerate() {
            let local_max = results.data.iter().max().unwrap();
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
            let local_min = results.data.iter().max().unwrap();
            if *local_min < global_min { 
                global_min = *local_min;
                input = self.inputs[idx];
            }
        }
        (global_min, input)
    }

    pub fn local_avg_f32(&self) -> Vec<f32> {
        self.data.iter().map(|d| {
            let len = d.data.len();
            d.data.iter().sum::<usize>() as f32 / len as f32
        }).collect()
    }

    pub fn local_avg_usize(&self) -> Vec<usize> {
        self.data.iter().map(|d| {
            let len = d.data.len();
            d.data.iter().sum::<usize>() / len
        }).collect()
    }


    /// Return a list of the minimum observed values across all results. 
    pub fn local_min(&self) -> Vec<usize> {
        self.data.iter().map(|d| *d.data.iter().min().unwrap()).collect()
    }

    /// Return a list of the maximum observed values across all results. 
    pub fn local_max(&self) -> Vec<usize> {
        self.data.iter().map(|d| *d.data.iter().max().unwrap()).collect()
    }

    pub fn local_minmax(&self) -> Vec<(usize, usize)> {
        self.data.iter().map(|d| *d.data.iter().min().unwrap())
            .zip(self.data.iter().map(|d| *d.data.iter().max().unwrap()))
            .collect()
    }


    pub fn local_min_first_nonzero(&self) -> Option<(usize, I)> { 
        let local_mins: Vec<usize> = self.data.iter()
            .map(|d| *d.data.iter().min().unwrap())
            .collect();

        let nz = local_mins.iter().enumerate().find(|(i,m)| **m != 0);
        if let Some((idx, min)) = nz { 
            return Some((*min, self.inputs[idx]));
        }
        return None;
    }



    pub fn local_min_pairwise_diff(&self) -> Vec<isize> {

        let local_mins: Vec<usize> = self.data.iter()
            .map(|d| *d.data.iter().min().unwrap())
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
    pub fn record(&mut self, event: E, input: I, data: MeasureResults) {
        if let Some(mut records) = self.data.get_mut(&event) {
            records.push(data, input);
        } else { 
            let mut res = EventResults::new();
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

    pub fn write_results_freq(&self) {
        for case_results in self.data.iter() {
            for (event, event_results) in case_results.data.iter() {
                let path = Self::generate_filename(
                    case_results, event, event_results
                );
                let mut f = std::fs::OpenOptions::new()
                    .write(true).create(true).truncate(true)
                    .open(&path).unwrap();

                println!("[*] Writing min/avg/max results to {}", path);
                writeln!(f, "# {}", event.as_desc().name()).unwrap();

                let one_counts = event_results.data.iter()
                    .map(|results| results.count(1));

                let iterator = event_results.inputs.iter()
                    .zip(one_counts);
                for ((input, cnt)) in iterator {
                    writeln!(f, "input={} cnt={}",
                        input, cnt
                    ).unwrap();
                }
            }
        }

    }

    // NOTE: Temporary hack for dumping results to disk
    pub fn write_results(&self) {
        for case_results in self.data.iter() {
            for (event, event_results) in case_results.data.iter() {
                let path = Self::generate_filename(
                    case_results, event, event_results
                );
                let mut f = std::fs::OpenOptions::new()
                    .write(true).create(true).truncate(true)
                    .open(&path).unwrap();

                println!("[*] Writing min/avg/max results to {}", path);
                writeln!(f, "# {}", event.as_desc().name()).unwrap();
                let minmax = event_results.local_minmax();
                let avgs = event_results.local_avg_usize();
                let iterator = event_results.inputs.iter()
                    .zip(avgs.iter()).zip(minmax.iter());
                for ((input, avg), (min, max)) in iterator {
                    writeln!(f, "input={} min={} avg={} max={}", 
                        input, min, avg, max
                    ).unwrap();
                }
            }
        }
    }
}
