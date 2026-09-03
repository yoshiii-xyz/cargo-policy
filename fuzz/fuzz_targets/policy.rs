#![no_main]

use cargo_policy::Policy;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = text.parse::<toml::Value>();
        let _ = toml::from_str::<Policy>(text);
    }
});
