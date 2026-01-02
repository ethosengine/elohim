//! Custom getrandom v0.3 backend for native builds
//!
//! The holochain_client and its dependencies compile with getrandom's custom
//! backend enabled (intended for WASM). For native Linux builds, we delegate
//! to the standard getrandom syscall.
//!
//! This is needed because holochain_nonce, rand, and tungstenite are all
//! compiled expecting __getrandom_v03_custom to exist.

use std::io::Read;

/// Custom getrandom implementation required by holochain dependencies
///
/// Safety: This function is called by getrandom's custom backend.
/// On Linux, we use /dev/urandom for cryptographic randomness.
#[no_mangle]
pub unsafe extern "C" fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> u32 {
    if dest.is_null() || len == 0 {
        return 0;
    }

    let slice = std::slice::from_raw_parts_mut(dest, len);

    // Use /dev/urandom - always available, non-blocking, cryptographically secure
    match std::fs::File::open("/dev/urandom") {
        Ok(mut file) => {
            match file.read_exact(slice) {
                Ok(()) => 0, // Success
                Err(_) => 1, // Error - getrandom convention
            }
        }
        Err(_) => 1, // Error
    }
}
