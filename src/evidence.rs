use std::{ffi::CString, marker::PhantomData, ops::Deref};

use crate::fiftyone_degrees::{self, fiftyone_degrees_array_fiftyoneDegreesEvidenceKeyValuePair_t};

pub struct Evidence {
    data: EvidenceCollection,
}

enum EvidenceCollection {
    Empty,
    UserAgentOnly(CString),
    EvidenceKeyValues(Vec<EvidenceItem>),
}

struct EvidenceItem {
    kind: EvidenceKind,
    field: CString,
    value: CString,
}

pub enum EvidenceKind {
    HeaderString,
    HeaderIPAddresses,
    Server,
    Query,
    Cookie,
}

impl Default for Evidence {
    fn default() -> Self {
        Self {
            data: EvidenceCollection::Empty,
        }
    }
}

impl Evidence {
    pub fn add<T: AsRef<str>>(&mut self, kind: EvidenceKind, field: T, value: T) {
        let field: CString = CString::new(field.as_ref()).expect("error creating c string");
        let value: CString = CString::new(value.as_ref()).expect("error creating c string");
        self.data.add_evidence(kind, field, value);
    }

    pub fn new_with_user_agent<T: AsRef<str>>(ua: T) -> Self {
        let ua: CString = CString::new(ua.as_ref()).expect("error creating c string");
        Self {
            data: EvidenceCollection::UserAgentOnly(ua),
        }
    }

    pub fn len(&self) -> usize {
        match &self.data {
            EvidenceCollection::Empty => 0,
            EvidenceCollection::UserAgentOnly(_) => 1,
            EvidenceCollection::EvidenceKeyValues(kvs) => kvs.len(),
        }
    }
}

impl EvidenceKind {
    fn to_prefix(&self) -> fiftyone_degrees::EvidencePrefix {
        match self {
            Self::HeaderString => fiftyone_degrees::e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_STRING,
            Self::HeaderIPAddresses => fiftyone_degrees::e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_IP_ADDRESSES,
            Self::Server => fiftyone_degrees::e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_SERVER,
            Self::Query=> fiftyone_degrees::e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_QUERY,
            Self::Cookie => fiftyone_degrees::e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_COOKIE
        }
    }
}

impl EvidenceCollection {
    fn add_evidence(&mut self, kind: EvidenceKind, field: CString, value: CString) {
        *self = match std::mem::replace(self, Self::Empty) {
            Self::Empty => Self::EvidenceKeyValues(vec![EvidenceItem { kind, field, value }]),
            Self::UserAgentOnly(ua) => {
                let mut values = Self::new_key_values_with_ua(ua);
                values.push(EvidenceItem { kind, field, value });
                Self::EvidenceKeyValues(values)
            }
            Self::EvidenceKeyValues(mut values) => {
                values.push(EvidenceItem { kind, field, value });
                Self::EvidenceKeyValues(values)
            }
        }
    }

    fn new_key_values_with_ua(user_agent: CString) -> Vec<EvidenceItem> {
        vec![EvidenceItem {
            kind: EvidenceKind::HeaderString,
            field: c"user-agent".to_owned(),
            value: user_agent,
        }]
    }
}

pub struct FiftyoneDegreesKeyValueArray<'a> {
    pub kv_array: *mut fiftyone_degrees_array_fiftyoneDegreesEvidenceKeyValuePair_t,
    backing_store: PhantomData<&'a Evidence>,
}

impl Drop for FiftyoneDegreesKeyValueArray<'_> {
    fn drop(&mut self) {
        unsafe {
            fiftyone_degrees::fiftyoneDegreesEvidenceFree(self.kv_array);
        }
    }
}

impl<'a> FiftyoneDegreesKeyValueArray<'a> {
    pub fn new(evidence: &'a Evidence) -> Self {
        let kv_array =
            unsafe { fiftyone_degrees::fiftyoneDegreesEvidenceCreate(evidence.len() as u32) };
        // TODO: check for null pointer
        match &evidence.data {
            EvidenceCollection::Empty => {}
            EvidenceCollection::UserAgentOnly(ua) => unsafe {
                fiftyone_degrees::fiftyoneDegreesEvidenceAddString(
                    kv_array, 
                    fiftyone_degrees::e_fiftyone_degrees_evidence_prefix_FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_STRING, 
                    c"user-agent".as_ptr(), 
                    ua.as_ptr()
                );
            },
            EvidenceCollection::EvidenceKeyValues(kvs) => for item in kvs {
                unsafe {
                    fiftyone_degrees::fiftyoneDegreesEvidenceAddString(kv_array, item.kind.to_prefix(), item.field.as_ptr(), item.value.as_ptr());
                };
            }

        }
        Self {
            kv_array,
            backing_store: PhantomData::default(),
        }
    }
}
