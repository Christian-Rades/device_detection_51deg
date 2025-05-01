use anyhow::Result;
use std::{path::PathBuf, str::FromStr};

use device_detection_51deg::{
    evidence::{Evidence, EvidenceKind},
    hash_engine::HashEngineBuilder,
};

fn demo_evidence() -> Vec<Evidence> {
    vec![
        // A User-Agent from a mobile device.
        Evidence::new_with_user_agent(
            "Mozilla/5.0 (Linux; Android 9; SAMSUNG SM-G960U AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/10.1 Chrome/71.0.3578.99 Mobile Safari/537.36",
        ),
        // A User-Agent from a desktop device.
        Evidence::new_with_user_agent(
            "Mozilla / 5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/78.0.3904.108 Safari/537.36",
        ),
        // Evidence values from a windows 11 device using a browser
        // that supports User-Agent Client Hints.
        Evidence::new_with_user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.102 Safari/537.36",
        )
        .add(EvidenceKind::HeaderString, "sec-ch-ua-mobile", "?0")
        .add(EvidenceKind::HeaderString, "sec-ch-ua", "\" Not A; Brand\";v=\"99\", \"Chromium\";v=\"98\", \"Google Chrome\";v=\"98\"")
        .add(EvidenceKind::HeaderString, "sec-ch-ua-platform", "Windows")
        .add(EvidenceKind::HeaderString, "sec-ch-ua-platform-version", "\"14.0.0\""),
        Evidence::new_with_user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.102 Safari/537.36"
        )
        .add(EvidenceKind::HeaderString,  "sec-ch-ua-mobile", "?0")
        .add(EvidenceKind::HeaderString,  "sec-ch-ua", "\" Not A; Brand\";v=\"99\", \"Chromium\";v=\"98\", \"Google Chrome\";v=\"98\"")
        .add(EvidenceKind::HeaderString,   "sec-ch-ua-platform", "Windows")
        .add(EvidenceKind::HeaderString,    "sec-ch-ua-platform-version", "\"14.0.0\""),
    ]
}

fn main() -> Result<()> {
    let hash_file =
        PathBuf::from_str("device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash")?;

    let manager = HashEngineBuilder::new(&hash_file)
        .hash_config(device_detection_51deg::hash_engine::HashConfig::HighPerformance)
        .init()?;

    let mut device_id = String::default();
    for (i, mut evidence) in demo_evidence().into_iter().enumerate() {
        if i == 3 {
            evidence = evidence.add(EvidenceKind::Query, "51D_deviceId", &device_id);
        }

        let mut result = manager.process(&evidence)?;

        if i == 0 {
            device_id = result.get_device_id().unwrap();
        }

        println!("Input:");
        println!("\t {:?}", evidence);
        println!("Results:");
        result
            .get_str("IsMobile")
            .inspect(|s| println!("\tMobile Device: {}", s));

        result
            .get_str("PlatformName")
            .inspect(|s| println!("\tPlatform Name: {}", s));
        result
            .get_str("PlatformVersion")
            .inspect(|s| println!("\tPlatform Version {:?}", s));
        result
            .get_str("BrowserName")
            .inspect(|s| println!("\tBrowser Name: {}", s));
        result
            .get_str("BrowserVersion")
            .inspect(|s| println!("\tBrowser Version: {}", s));
    }
    Ok(())
}
