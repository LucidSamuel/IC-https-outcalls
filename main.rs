use candid::Principal;
use exchange_rate::{Rate, RatesWithInterval, TimeRange, Timestamp};
use ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformFunc, TransformType,
};
use ic_cdk::storage;
use ic_cdk_macros::{self, heartbeat, post_upgrade, pre_upgrade, query, update};
use serde_json::{self, Value};
use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};

// How many data point can be returned as maximum.
// Given that 2MB is max-allow canister response size, and each <Timestamp, Rate> pair
// should be less that 20 bytes. Maximum data points could be returned for each
// call can be as many as 2MB / 20B = 100000.
pub const MAX_DATA_PONTS_CANISTER_RESPONSE: usize = 100000;

// Remote fetch interval in secs. It is only the canister returned interval
// that is dynamic according to the data size needs to be returned.
pub const REMOTE_FETCH_GRANULARITY: u64 = 60;

// For how many rounds of heartbeat, make a http_request call.
pub const RATE_LIMIT_FACTOR: usize = 5;

// How many data points in each Coinbase API call. Maximum allowed is 300
pub const DATA_POINTS_PER_API: u64 = 200;

// Maximum raw Coinbase API response size. This field is used by IC to calculate circles cost per HTTP call.
// Here is how this number is derived:
// Each Coinbase API call return an array of array, and each sub-array look like below:
// [
//     1652454000,
//     9.51,
//     9.6,
//     9.55,
//     9.54,
//     4857.1892
// ],
// Each field of this sub-arry takes less than 10 bytes. Then,
// 10 (bytes per field) * 6 (fields per timestamp) * 200 (timestamps)
pub const MAX_RESPONSE_BYTES: u64 = 10 * 6 * DATA_POINTS_PER_API;

thread_local! {
    pub static FETCHED: RefCell<HashMap<Timestamp, Rate>>  = RefCell::new(HashMap::new());
    pub static REQUESTED: RefCell<HashSet<Timestamp>> = RefCell::new(HashSet::new());
    pub static RATE_COUNTER: RefCell<usize> = RefCell::new(0);
}
