# Rust wraper for the 51 Degrees device detection
This wrapper allows the creation and usage of the 51 Degrees device detection hash engine.
For more information about 51 Degrees see: https://51degrees.com/documentation/4.4/index.html
## Getting started:
```
 // Lite hash for demo purposes
 let file: PathBuf = "51Degrees-LiteV4.1.hash".into();
 let manager = HashEngineBuilder::new(&file)
     .hash_config(HashConfig::HighPerformance)
     .init()
     .unwrap();
 let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Mobile/15E148 Safari/604.1";

 let evidence = Evidence::new_with_user_agent(ua);
 let mut results = manager.process(&evidence).unwrap();
 let res = results.get_str("PlatformName");
 assert_eq!(res, Some("iOS"));
```
## Configuration
Currently switching between hash configs and defining a list of result properties is
implemented.
Peformance can be switched between:
- HighPerformance
- InMemory
- Balanced
- LowMemory

With the first config being the fastest at the expense of memory footprint and
the last one being the opposite.

Properties are a list of static strings that define the values that are put into the result
during the processing of the evidence.
Limiting the device properties in the result can help speed up the processing of the evidence.
