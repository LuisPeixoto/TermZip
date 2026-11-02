//! The files of a cloned project: the readers return plain names or nothing, never a panic.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    for name in termzip_tasks::package_scripts(&text)
        .into_iter()
        .chain(termzip_tasks::deno_tasks(&text))
        .chain(termzip_tasks::cargo_aliases(&text))
        .chain(termzip_tasks::makefile_targets(&text))
        .chain(termzip_tasks::justfile_recipes(&text))
    {
        assert!(termzip_tasks::is_plain_name(&name), "a name that is not plain was offered: {name:?}");
    }
    let _ = termzip_tasks::package_manager_field(&text);
});
