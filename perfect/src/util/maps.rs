//! Module for interacting with `/proc/self/maps`. 

use std::ops::Range;

/// Wrapper for interacting with '/proc/self/maps'.
pub struct Maps;
impl Maps { 

    /// Return a list of virtual memory regions which are mapped. 
    pub fn ranges() -> Result<Vec<Range<usize>>, &'static str> { 
        use std::io::prelude::*;
        use std::io::BufReader;
        let mut f = std::fs::File::open("/proc/self/maps").map_err(|_| { 
            "Couldn't open /proc/self/maps (do you have permission?)"
        })?;

        let mut ranges = Vec::new();
        let mut range: Option<Range<usize>> = None;
        for line in BufReader::new(f).lines() { 
            let line = line.unwrap();
            let x = line.split(" ").take(1).collect::<String>();
            let x = x.split("-").collect::<Vec<&str>>();
            let start = usize::from_str_radix(x[0], 16).unwrap();
            let end = usize::from_str_radix(x[1], 16).unwrap();

            // Merge with the previous range if possible
            if let Some(r) = &mut range {
                if r.end == start { 
                    r.end = end;
                } else { 
                    ranges.push(r.clone());
                    *r = start..end;
                }
            } else { 
                range = Some(start..end);
            }
        }
        ranges.push(range.unwrap());
        Ok(ranges)
    }



}

