#![no_std]

use soroban_sdk::contractmeta;

mod constants;
mod contract;
mod deposit;
mod errors;
mod events;
mod storage;
mod tests;
mod utils;

contractmeta!(key = "version", val = "1.0.0");
