use std::{
    cell::UnsafeCell,
    error::Error,
    ffi::{CStr, CString},
    fmt::{Display, Write},
    mem,
    path::{Path, PathBuf},
};

use crate::{
    evidence::{Evidence, EvidenceRef},
    fiftyone_degrees::{
        self, Exception, ResourceManager, fiftyone_degrees_string_t,
        fiftyoneDegreesHashGetDeviceIdFromResults, fiftyoneDegreesResultsHashCreate,
        fiftyoneDegreesResultsHashFree, fiftyoneDegreesResultsHashFromEvidence,
        fiftyoneDegreesResultsHashGetValues,
    },
};

#[derive(Clone, Copy, Debug)]
pub enum HashConfig {
    InMemory,
    HighPerformance,
    LowMemory,
    SingleLoaded,
}

/// A builder to configure the hash engine.
/// A a path to the hash file is mandatory.
/// The performance configuration can be set with the `hash_config` function,
/// `LowMemory` is the default.
///
pub struct HashEngineBuilder {
    hash_config: HashConfig,
    hash_file: PathBuf,
    properties: Vec<&'static str>,
}

/// A wrapper type for the hash device detection.
/// The engine provides a way to lookup devices based on Evidence.
pub struct HashEngine {
    manager: Box<UnsafeCell<ResourceManager>>,
    _properties: CString,
}

impl Drop for HashEngine {
    fn drop(&mut self) {
        unsafe {
            fiftyone_degrees::fiftyoneDegreesResourceManagerFree(self.manager.get_mut());
        }
    }
}

impl HashEngineBuilder {
    /// Creates a new builder based on the location of the device_detection.hash file.
    pub fn new(hash_file: &Path) -> Self {
        Self {
            hash_config: HashConfig::LowMemory,
            hash_file: hash_file.to_owned(),
            properties: Vec::default(),
        }
    }

    /// Sets the performance configuration of the hash engine.
    /// Defaults to `LowMemory`
    pub fn hash_config(mut self, config: HashConfig) -> Self {
        self.hash_config = config;
        self
    }

    /// Sets the device properties that are returned by the hash engine.
    /// Defaults to all properties available.
    /// See: [51Degrees Docs](https://51degrees.com/device-detection-cxx/4.4/group___fifty_one_degrees_properties.html#gafe718e9dd0c8b93c755337a6f17b2b60)
    ///
    pub fn set_properties(mut self, properties: &[&'static str]) -> Self {
        self.properties = properties.to_vec();
        self
    }

    /// Allocates and initializes the hash engine.
    pub fn init(self) -> Result<HashEngine, HashManagerError> {
        let data_file = CString::new(self.hash_file.as_os_str().as_encoded_bytes())
            .expect("path to cstring conversion failed");

        let mut buf: String = String::default();

        for item in &self.properties {
            if !buf.is_empty() {
                buf.write_str(",").expect("writing to property buffer");
            }

            buf.write_str(item).expect("writing to property buffer");
        }

        let mut manager = Box::new(UnsafeCell::new(unsafe { mem::zeroed::<ResourceManager>() }));
        let properties = CString::new(buf).expect("allocating a new properties string");

        let mut exception = fiftyone_degrees::Exception::default();
        let status = unsafe {
            let mut default = fiftyone_degrees::fiftyoneDegreesPropertiesDefault;

            if !self.properties.is_empty() {
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
                manager.get_mut(),
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

        Ok(HashEngine {
            manager,
            _properties: properties,
        })
    }
}

impl<'a> HashEngine {
    /// Allocates and fills a result with the evidence provided.
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

        let evidence_ref = EvidenceRef::new(evidence);
        let mut exception = Exception::default();
        unsafe {
            fiftyoneDegreesResultsHashFromEvidence(
                result_ptr,
                evidence_ref.kv_array,
                &mut exception,
            )
        };

        if !exception.is_ok() {
            return Err(HashManagerError {
                kind: HashManagerErrorKind::Process(exception),
            });
        }

        Ok(ResultsHash {
            result_ptr,
            manager: self,
        })
    }

    fn get_property_index(&self, property: &'static str) -> i32 {
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

        index
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
                "error initializing the fiftyoneDegrees resource manager."
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

/// A wrapper type for the ResultsHash provided by the hash engine.
pub struct ResultsHash<'a> {
    result_ptr: *mut fiftyone_degrees::ResultsHash,
    manager: &'a HashEngine,
}

impl Drop for ResultsHash<'_> {
    fn drop(&mut self) {
        unsafe {
            fiftyoneDegreesResultsHashFree(self.result_ptr);
        }
    }
}

impl<'a, 'b> ResultsHash<'a>
where
    'a: 'b,
{
    /// Looks up the 51 Degrees device ID and returns a copy.
    pub fn get_device_id(&'b mut self) -> Option<String> {
        let mut id_buf = [0u8; 512];
        let mut exception = Exception::default();
        unsafe {
            fiftyoneDegreesHashGetDeviceIdFromResults(
                self.result_ptr,
                id_buf.as_mut_ptr() as *mut i8,
                id_buf.len(),
                &mut exception,
            ) as usize
        };

        if !exception.is_ok() {
            return None;
        }
        let Ok(ids) = CStr::from_bytes_until_nul(&id_buf) else {
            return None;
        };
        ids.to_str().ok().map(str::to_string)
    }

    /// Returns a reference to the value of the given property.
    /// Returns None in case the property does not exist or the engine was configured
    /// to ignore the requested property.
    pub fn get_str(&'b mut self, property: &'static str) -> Option<&'b str> {
        let index = self.manager.get_property_index(property);
        let mut exception = Exception::default();
        let collection =
            unsafe { fiftyoneDegreesResultsHashGetValues(self.result_ptr, index, &mut exception) };

        if !exception.is_ok() {
            return None;
        }
        if collection.is_null() {
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
    use crate::evidence::Evidence;

    use super::*;

    #[test]
    fn smoke_test() {
        let file: PathBuf =
            "device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash".into();
        let manager = HashEngineBuilder::new(&file)
            .hash_config(HashConfig::HighPerformance)
            .init()
            .unwrap();
        let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Mobile/15E148 Safari/604.1";

        let evidence = Evidence::new_with_user_agent(ua);
        let mut results = manager.process(&evidence).unwrap();
        let res = results.get_str("PlatformName");
        assert_eq!(res, Some("iOS"));
    }

    #[test]
    fn custom_properties() {
        let file: PathBuf =
            "device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash".into();
        let manager = HashEngineBuilder::new(&file)
            .hash_config(HashConfig::HighPerformance)
            .set_properties(&["IsMobile"])
            .init()
            .unwrap();
        let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Mobile/15E148 Safari/604.1";

        let evidence = Evidence::new_with_user_agent(ua);
        let mut results = manager.process(&evidence).unwrap();
        let res = results.get_str("IsMobile");
        assert_eq!(res, Some("True"));
    }
}
