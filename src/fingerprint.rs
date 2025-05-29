use serde::Deserialize;
use wasm_fingerprint::make_fingerprint;
use web_sys::wasm_bindgen::UnwrapThrowExt;

pub fn fingerprint() -> String {
    serde_json::from_str::<Fingerprint>(&make_fingerprint().unwrap_throw())
        .unwrap_throw()
        .print
}

#[derive(Deserialize)]
struct Fingerprint {
    print: String,
}
