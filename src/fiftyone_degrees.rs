#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
// bindgen issue? https://github.com/rust-lang/rust-bindgen/issues/3147
#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    error::{self, Error},
    ffi::{CStr, CString, c_void},
    fmt::{Debug, Display},
    ptr,
};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl Default for Exception {
    fn default() -> Self {
        fiftyone_degrees_exception_t {
            file: ptr::null(),
            func: ptr::null(),
            line: -1,
            status: e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET,
        }
    }
}

impl Display for Exception {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let exception = ptr::from_ref(self);
        let mut msg =
        // c function does not write the exception memory, but is not marked *const T either
            unsafe { CStr::from_ptr(fiftyoneDegreesExceptionGetMessage(exception.cast_mut())) };

        let result = f.write_str(msg.to_str().unwrap_or("error formatting error message"));

        unsafe {
            // caller must free the string allocated in the c function
            free(msg.as_ptr() as *mut c_void);
        };
        result
    }
}

impl Exception {
    pub fn is_ok(&self) -> bool {
        self.status == e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET
    }
}

impl Error for Exception {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str;
    use std::ffi::CStr;
    use std::mem::MaybeUninit;
    use std::ptr;

    #[test]
    fn smoke_test() {
        unsafe {
            let mut manager = MaybeUninit::<ResourceManager>::uninit();
            let mut exception = Exception::default();
            let mut default = fiftyoneDegreesPropertiesDefault;
            let mut config = fiftyoneDegreesHashHighPerformanceConfig;
            let data_file = c"device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash";

            let status = fiftyoneDegreesHashInitManagerFromFile(
                manager.as_mut_ptr(),
                &mut config,
                &mut default,
                data_file.as_ptr(),
                &mut exception,
            );
            let mut manager = manager.assume_init();
            assert_eq!(status, EXIT_SUCCESS);
            assert_eq!(
                exception.status,
                e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET
            );
            assert!(!manager.active.is_null());

            let results = fiftyoneDegreesResultsHashCreate(&mut manager, 1, 0);
            let evidence = fiftyoneDegreesEvidenceCreate(1);

            let name = c"user-agent";
            let ua = c"Mozilla/5.0 (iPhone; CPU iPhone OS 16_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Mobile/15E148 Safari/604.1";

            fiftyoneDegreesEvidenceAddString(
                evidence,
                e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_STRING,
                name.as_ptr(),
                ua.as_ptr(),
            );

            let mut exception = Exception {
                file: ptr::null(),
                func: ptr::null(),
                line: -1,
                status: e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET,
            };

            fiftyoneDegreesResultsHashFromEvidence(results, evidence, &mut exception);
            assert_eq!(
                exception.status,
                e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET
            );

            let dataset_ref = fiftyoneDegreesDataSetGet(&mut manager);
            let property_name = c"PlatformName";
            let property_index = fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName(
                (*dataset_ref).available,
                property_name.as_ptr(),
            );
            fiftyoneDegreesDataSetRelease(dataset_ref);
            assert!(
                fiftyoneDegreesResultsHashGetHasValues(results, property_index, &mut exception),
                "{:?}",
                CStr::from_ptr(fiftyoneDegreesResultsHashGetNoValueReasonMessage(
                    fiftyoneDegreesResultsHashGetNoValueReason(
                        results,
                        property_index,
                        &mut exception
                    )
                ))
            );

            let mut value_buffer: [u8; 1024] = [0; 1024];

            let n = fiftyoneDegreesResultsHashGetValuesString(
                results,
                property_name.as_ptr(),
                value_buffer.as_mut_ptr() as *mut i8,
                value_buffer.len(),
                c",".as_ptr(),
                &mut exception,
            );

            assert_eq!(
                exception.status,
                e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET
            );

            let value = str::from_utf8(&value_buffer[0..n]).unwrap();
            assert_eq!(value, "iOS");
        }
    }
}
