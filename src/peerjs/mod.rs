use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub mod client;
pub mod dataconnection;
pub mod ffi;
pub mod peerid;

const CHANNEL_BUFFER_SIZE: usize = 100;

#[wasm_bindgen]
#[derive(Debug, Serialize, Deserialize)]
pub struct ICEServer {
    urls: String,
    username: Option<String>,
    credential: Option<String>,
}

#[wasm_bindgen]
impl ICEServer {
    pub fn new(urls: String, username: Option<String>, credential: Option<String>) -> ICEServer {
        ICEServer {
            urls,
            username,
            credential,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn urls(&self) -> String {
        self.urls.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn username(&self) -> Option<String> {
        self.username.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn credential(&self) -> Option<String> {
        self.credential.clone()
    }
}
