#![no_std]

use soroban_sdk::contractmeta;

pub mod constants;
pub mod contract;
pub mod errors;
pub mod events;
pub mod rewards;
pub mod storage;
mod tests;

contractmeta!(key = "version", val = "1.0.0");
