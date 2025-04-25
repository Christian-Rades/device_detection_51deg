use std::{
    cell::UnsafeCell,
    error::Error,
    ffi::{CStr, CString},
    fmt::{Display, Write},
    mem::MaybeUninit,
    path::{Path, PathBuf},
};

use crate::{
    evidence::{Evidence, FiftyoneDegreesKeyValueArray},
    fiftyone_degrees::{
        self, Exception, ResourceManager, fiftyone_degrees_string_t,
        fiftyoneDegreesResultsHashCreate, fiftyoneDegreesResultsHashFree,
        fiftyoneDegreesResultsHashFromEvidence, fiftyoneDegreesResultsHashGetValues,
    },
};

#[derive(Clone, Copy, Debug)]
pub enum HashConfig {
    InMemory,
    HighPerformance,
    LowMemory,
    SingleLoaded,
}

pub struct HashManagerBuilder {
    hash_config: HashConfig,
    hash_file: PathBuf,
    properties: Vec<&'static str>,
}

pub struct HashManager {
    manager: UnsafeCell<ResourceManager>,
    properties: CString,
}

impl Drop for HashManager {
    fn drop(&mut self) {
        dbg!("drop manager");
        let mut p = self.manager.into_inner();
        unsafe {
            fiftyone_degrees::fiftyoneDegreesResourceManagerFree(&mut p);
        }
    }
}

impl HashManagerBuilder {
    pub fn new(hash_file: &Path) -> Self {
        Self {
            hash_config: HashConfig::LowMemory,
            hash_file: hash_file.to_owned(),
            properties: Vec::default(),
        }
    }

    pub fn hash_config(mut self, config: HashConfig) -> Self {
        self.hash_config = config;
        self
    }

    pub fn init(self) -> Result<HashManager, HashManagerError> {
        let mut manager = MaybeUninit::<ResourceManager>::uninit();
        let mut exception = fiftyone_degrees::Exception::default();
        let data_file = CString::new(self.hash_file.as_os_str().as_encoded_bytes())
            .expect("path to cstring conversion failed");

        let mut buf: String = String::default();

        for item in &self.properties {
            if buf.len() > 0 {
                buf.write_str(",").expect("writing to property buffer");
            }

            buf.write_str(item).expect("writing to property buffer");
        }

        let properties = CString::new(buf).expect("allocating a new properties string");

        let status = unsafe {
            let mut default = fiftyone_degrees::fiftyoneDegreesPropertiesDefault;

            if self.properties.len() > 0 {
                default.string = properties.as_ptr();
            }

            let mut config = match self.hash_config {
                HashConfig::LowMemory => fiftyone_degrees::fiftyoneDegreesHashLowMemoryConfig,
                HashConfig::InMemory => fiftyone_degrees::fiftyoneDegreesHashInMemoryConfig,
                HashConfig::HighPerformance => {
                    fiftyone_degrees::fiftyoneDegreesHashHighPerformanceConfig
                }
                HashConfig::SingleLoaded => fiftyone_degrees::fiftyoneDegreesHashSingleLoadedConfig,
            };

            fiftyone_degrees::fiftyoneDegreesHashInitManagerFromFile(
                manager.as_mut_ptr(),
                &mut config,
                &mut default,
                data_file.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_ok() {
            return Err(HashManagerError {
                kind: HashManagerErrorKind::Init(exception),
            });
        }

        if status != fiftyone_degrees::EXIT_SUCCESS {
            return Err(HashManagerError {
                kind: HashManagerErrorKind::WithoutException(ErrStatus { status }),
            });
        }

        Ok(HashManager {
            manager: unsafe { manager.assume_init()) },
            properties,
        })
    }
}

impl<'a> HashManager {
    pub fn process(&'a self, evidence: &'_ Evidence) -> Result<ResultsHash<'a>, HashManagerError> {
        let max_len = evidence.len() as u32;
        let result_ptr = unsafe {
            fiftyoneDegreesResultsHashCreate(self.manager.get().cast(), max_len, max_len)
        };
        if result_ptr.is_null() {
            return Err(HashManagerError {
                kind: HashManagerErrorKind::AllocatingResult,
            });
        }

        let kv_array = FiftyoneDegreesKeyValueArray::new(evidence);
        let mut exception = Exception::default();
        unsafe {
            fiftyoneDegreesResultsHashFromEvidence(result_ptr, kv_array.kv_array, &mut exception)
        };

        if !exception.is_ok() {
            return Err(HashManagerError {
                kind: HashManagerErrorKind::Process(exception),
            });
        }

        return Ok(ResultsHash {
            result_ptr,
            manager: self,
        });
    }

    pub fn get_property_index(&self, property: &'static str) -> i32 {
        let dataset_ref =
            unsafe { fiftyone_degrees::fiftyoneDegreesDataSetGet(self.manager.get().cast()) };

        let c_name = CString::new(property).expect("static string to cstring");
        let index = unsafe {
            fiftyone_degrees::fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName(
                (*dataset_ref).available,
                c_name.as_ptr(),
            )
        };

        unsafe {
            fiftyone_degrees::fiftyoneDegreesDataSetRelease(dataset_ref);
        }
        return index;
    }
}

#[derive(Debug)]
pub struct HashManagerError {
    kind: HashManagerErrorKind,
}

impl Display for HashManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            HashManagerErrorKind::Init(_) | HashManagerErrorKind::WithoutException(_) => write!(
                f,
                "error initializeing the fiftyoneDegrees resource manager."
            ),
            HashManagerErrorKind::AllocatingResult | HashManagerErrorKind::Process(_) => {
                write!(f, "error proccessing the evidence")
            }
        }
    }
}

impl Error for HashManagerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            HashManagerErrorKind::AllocatingResult => None,
            HashManagerErrorKind::Init(e) => Some(e),
            HashManagerErrorKind::Process(e) => Some(e),
            HashManagerErrorKind::WithoutException(status) => Some(status),
        }
    }
}

#[derive(Debug)]
pub enum HashManagerErrorKind {
    AllocatingResult,
    Init(Exception),
    Process(Exception),
    WithoutException(ErrStatus),
}

#[derive(Debug)]
pub struct ErrStatus {
    status: u32,
}

impl Display for ErrStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "init status code: {}.", self.status)
    }
}
impl Error for ErrStatus {}

pub struct ResultsHash<'a> {
    result_ptr: *mut fiftyone_degrees::ResultsHash,
    manager: &'a HashManager,
}

impl Drop for ResultsHash<'_> {
    fn drop(&mut self) {
        unsafe {
            dbg!("drop result");
            fiftyoneDegreesResultsHashFree(self.result_ptr);
        }
    }
}

impl<'a, 'b> ResultsHash<'a>
where
    'a: 'a,
{
    pub fn get_value_as_str(&'b self, property: &'static str) -> Option<&'b str> {
        let index = self.manager.get_property_index(property);
        let mut exception = Exception::default();
        let collection =
            unsafe { fiftyoneDegreesResultsHashGetValues(self.result_ptr, index, &mut exception) };
        if collection.is_null() {
            return None;
        }
        if !exception.is_ok() {
            return None;
        }
        if unsafe { (*self.result_ptr).values.count } == 0 {
            return None;
        }
        unsafe {
            let items = (*self.result_ptr).values.items;
            let str_data = (*items).data.ptr as *mut fiftyone_degrees_string_t;
            CStr::from_ptr(std::ptr::from_ref(&(*str_data).value))
                .to_str()
                .ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::evidence::{Evidence, FiftyoneDegreesKeyValueArray};

    use super::*;

    #[test]
    fn smoke_test() {
        let file: PathBuf =
            "device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash".into();
        let manager = HashManagerBuilder::new(&file)
            .hash_config(HashConfig::HighPerformance)
            .init()
            .unwrap();
        let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Mobile/15E148 Safari/604.1";

        let evidence = Evidence::new_with_user_agent(ua);
        let results = manager.process(&evidence).unwrap();
        let res = results.get_value_as_str("PlatformName");
        assert_eq!(res, Some("iOS"));
        dbg!(res);
        assert!(manager.get_property_index("PlatformName") > 0);
    }
}
