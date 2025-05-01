//! # Rust wraper for the 51 Degrees device detection
//! This wrapper allows the creation and usage of the 51 Degrees device detection hash engine.
//! ## Getting started:
//! ```no_run
//! use std::{path::PathBuf, str::FromStr};
//!
//! use device_detection_51deg::{
//!     evidence::{Evidence, EvidenceKind},
//!     hash_engine::{HashConfig, HashEngineBuilder},
//! };
//!
//! // Lite hash for demo purposes
//! let file: PathBuf = "51Degrees-LiteV4.1.hash".into();
//! let manager = HashEngineBuilder::new(&file)
//!     .hash_config(HashConfig::HighPerformance)
//!     .init()
//!     .unwrap();
//!
//! let evidence = Evidence::new_with_user_agent(
//! "Mozilla/5.0 (iPhone; CPU iPhone OS 16_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Mobile/15E148 Safari/604.1"
//! );
//!
//! let mut results = manager.process(&evidence).unwrap();
//! let res = results.get_str("PlatformName");
//!
//! assert_eq!(res, Some("iOS"));
//! ```
//! ## Configuration
//! Currently switching between hash configs and defining a list of result properties is
//! implemented.
//! Peformance can be switched between:
//! - HighPerformance
//! - InMemory
//! - Balanced
//! - LowMemory
//!
//! With the first config being the fastest at the expense of memory footprint and
//! the last being the opposite.
//!
//! Properties are a list of static strings that define the values that are put into the result
//! during the processing of the evidence.
//! Limiting the device properties in the result can help speed up the processing of the evidence.

pub mod evidence;
mod fiftyone_degrees;
pub mod hash_engine;

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};

    use crate::evidence::Evidence;

    use super::*;

    #[test]
    fn full_test() {
        let mut test_file = PathBuf::from_str(
            "device-detection-cxx/device-detection-data/20000 Evidence Records.yml",
        )
        .unwrap();

        let Ok(test_data) = fs::read_to_string(&test_file) else {
            // TODO: find a better way to split tests into fast and slow
            return;
        };

        let mut cases: Vec<HashMap<String, String>> = Vec::default();

        for part in test_data.split("---") {
            if part.is_empty() {
                continue;
            }
            cases.push(serde_yaml::from_str(part).unwrap());
        }
        let cases_evidence: Vec<Evidence> = cases
            .iter()
            .map(|record| {
                let mut evidence = Evidence::default();
                for (key, value) in record {
                    let field = key
                        .strip_prefix("header.")
                        .expect("all hints should be headers");
                    evidence = evidence.add(evidence::EvidenceKind::HeaderString, field, value)
                }
                evidence
            })
            .collect();

        let hash_engine = hash_engine::HashEngineBuilder::new(
            &PathBuf::from_str(
                "device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash",
            )
            .unwrap(),
        )
        .init()
        .inspect_err(|e| {
            dbg!(format!("{}", e));
        })
        .expect("building the engine should work");

        for evidence in cases_evidence {
            let mut result = hash_engine
                .process(&evidence)
                .expect("processing evidence to work");
            assert!(result.get_device_id().is_some_and(|id| id.len() > 0))
        }
    }
}
