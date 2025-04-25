#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod fiftyone_degrees;
pub mod hash_manager;
pub mod evidence;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

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
            let mut manager = MaybeUninit::<fiftyone_degrees::ResourceManager>::uninit();
            let mut exception = fiftyone_degrees::Exception {
                file: ptr::null(),
                func: ptr::null(),
                line: -1,
                status:
                    fiftyone_degrees::e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET,
            };
            let mut default = fiftyone_degrees::fiftyoneDegreesPropertiesDefault;
            let mut config = fiftyone_degrees::fiftyoneDegreesHashHighPerformanceConfig;
            let data_file = c"device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash";

            let status = fiftyone_degrees::fiftyoneDegreesHashInitManagerFromFile(
                manager.as_mut_ptr(),
                &mut config,
                &mut default,
                data_file.as_ptr(),
                &mut exception,
            );
            let mut manager = manager.assume_init();
            assert_eq!(status, fiftyone_degrees::EXIT_SUCCESS);
            assert_eq!(
                exception.status,
                fiftyone_degrees::e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET
            );
            assert!(!manager.active.is_null());

            let results = fiftyone_degrees::fiftyoneDegreesResultsHashCreate(&mut manager, 1, 0);
            let evidence = fiftyone_degrees::fiftyoneDegreesEvidenceCreate(1);

            let name = c"user-agent";
            let ua = c"Mozilla/5.0 (iPhone; CPU iPhone OS 16_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Mobile/15E148 Safari/604.1";


            fiftyone_degrees::fiftyoneDegreesEvidenceAddString(
                evidence, 
                fiftyone_degrees::e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_STRING, 
                name.as_ptr(),
                ua.as_ptr()
            );

            let mut exception = fiftyone_degrees::Exception {
                file: ptr::null(),
                func: ptr::null(),
                line: -1,
                status:
                    fiftyone_degrees::e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET,
            };

            fiftyone_degrees::fiftyoneDegreesResultsHashFromEvidence(results, evidence, &mut exception);
            assert_eq!(
                exception.status,
                fiftyone_degrees::e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET
            );

            let dataset_ref = fiftyone_degrees::fiftyoneDegreesDataSetGet(&mut manager);
            let property_name = c"PlatformName";
            let property_index = fiftyone_degrees::fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName((*dataset_ref).available, property_name.as_ptr());
            fiftyone_degrees::fiftyoneDegreesDataSetRelease(dataset_ref);
            assert!(fiftyone_degrees::fiftyoneDegreesResultsHashGetHasValues(results, property_index, &mut exception),"{:?}",
            CStr::from_ptr(fiftyone_degrees::fiftyoneDegreesResultsHashGetNoValueReasonMessage(fiftyone_degrees::fiftyoneDegreesResultsHashGetNoValueReason(results, property_index, &mut exception)))
            );

            let mut value_buffer: [u8; 1024]= [0; 1024];

            let n = fiftyone_degrees::fiftyoneDegreesResultsHashGetValuesString(results, property_name.as_ptr(), value_buffer.as_mut_ptr() as *mut i8, value_buffer.len(), c",".as_ptr(), &mut exception);

            assert_eq!(
                exception.status,
                fiftyone_degrees::e_fiftyone_degrees_status_code_FIFTYONE_DEGREES_STATUS_NOT_SET
            );

            let value = str::from_utf8(&value_buffer[0..n]).unwrap();
            assert_eq!(value, "iOS");
        }
    }
}
