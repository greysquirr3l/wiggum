#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let _ = wiggum::domain::plan::Plan::from_toml(input);
    let _ = wiggum::domain::workspace::Workspace::from_toml(input);
});
