//! # xdelta3
//!
//! This crate is a Rust binding of `xdelta3` which is written in C.
//!
//! In case you are not familar, `xdelta3` implements VCDIFF standard, which is a standard for
//! binary patches.
//! You can generate and apply VCDIFF patch for two similar (but large) binary files quickly using `xdelta3`.
//!
//! The original `xdelta3` utility is widely used for delivering software updates and ROM hacks.
//!
//! You can find out how to use this crate in this documentation and you can also consult the tests in the `test/` folder
//! to see it in action (how to generate and patch two files!)
use libc::c_uint;
use std::ffi::CStr;

#[cfg(feature = "stream")]
pub mod stream;

#[allow(dead_code)]
mod binding {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub enum Error {
    XDelta3 {
        error_code: i32,
    },
    InsufficientOutputLength,
    OutOfBounds {
        expected_length: u32,
        actual_length: u32,
    },
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::XDelta3 { error_code } => unsafe {
                let c_str = CStr::from_ptr(libc::strerror(*error_code));
                write!(f, "XDelta3 {} ({})", c_str.to_string_lossy(), error_code)
            },
            Self::InsufficientOutputLength => {
                write!(f, "InsufficientOutputLength")
            }
            Self::OutOfBounds {
                expected_length,
                actual_length,
            } => {
                write!(f, "OutOfBounds: {} > {}", actual_length, expected_length)
            }
        }
    }
}

/// Function to generate the difference data
///
/// This function is used to generate the difference data.
/// The data in `src` will be treated as "original" data and the
/// data in `input` will be treated as "after", "patched" or "expected" data.
///
/// If you want to build an application that applies patches or differential updates,
/// this function is used to generate the patch data (or update files).
/// When generating the patch file, you might want to read your old file into a `&[u8]` and
/// pass that variable to the `src` parameter and read your new file into another `&[u8]` and
/// pass that variable to the `input` parameter. And then you could write the output of this function
/// to a file.
///
/// Here is a basic example to show how to use this function:
/// ```
/// extern crate xdelta3;
/// use xdelta3::encode;
///
/// fn main() {
///     let result = encode(&[1, 2, 3, 4, 5, 6, 7], &[1, 2, 4, 4, 7, 6, 7]);
///     assert_eq!(result.unwrap().as_slice(), &[214, 195, 196, 0, 0, 0, 13, 7, 0, 7, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
/// }
/// ```
///
/// You might notice the generated patch data is larger than both orginal data and the updated data.
/// But don't worry, if your data is large enough and kind of similar between each other (usually the case
/// for software updates or ROM patches), the patch data should be only a fraction of your updated file.
pub fn encode(input: &[u8], src: &[u8]) -> Result<Vec<u8>, Error> {
    encode_with_output_len(input, src, (input.len() + src.len()) as u32 * 2)
}

pub fn encode_with_output_len(
    input: &[u8],
    src: &[u8],
    output_buffer_len: u32,
) -> Result<Vec<u8>, Error> {
    let input_len = input.len() as c_uint;
    let src_len = src.len() as c_uint;
    let mut avail_output = 0 as c_uint;
    let mut output = Vec::with_capacity(output_buffer_len as usize);
    let error_code = unsafe {
        binding::xd3_encode_memory(
            input.as_ptr(),
            input_len,
            src.as_ptr(),
            src_len,
            output.as_mut_ptr(),
            &mut avail_output,
            output_buffer_len,
            0,
        )
    };
    if error_code == 0 {
        // Extra sanity check to prevent UB.
        if avail_output > output_buffer_len {
            return Err(Error::OutOfBounds {
                expected_length: output_buffer_len,
                actual_length: avail_output,
            });
        }
        unsafe {
            output.set_len(avail_output as usize);
        }
        Ok(output)
    } else if error_code == libc::ENOSPC {
        Err(Error::InsufficientOutputLength)
    } else {
        Err(Error::XDelta3 { error_code })
    }
}

/// Function to decode the difference data
///
/// This function is used to decode the difference data.
/// The data in `src` will be treated as "original" data and the
/// data in `input` will be treated as "difference" or "patch" data.
/// The returned `Vec` stores the data that has been patched
///
/// As opposed to the encode function, if you are building an application that
/// applies patches or differential updates, this function should be used to
/// patch or update the old file from the patch data. It's recommeded to check
/// for the file integrity after doing the decode to prevent from creating
/// potentially corrupted files
///
/// Here is a basic example to show how to use this function:
/// ```
/// extern crate xdelta3;
/// use xdelta3::decode;
///
/// fn main() {
///     let result = decode(&[214, 195, 196, 0, 0, 0, 13, 7, 0, 7, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8], &[1, 2, 4, 4, 7, 6, 7]);
///     assert_eq!(result.unwrap().as_slice(), &[1, 2, 3, 4, 5, 6, 7]);
/// }
/// ```
pub fn decode(input: &[u8], src: &[u8]) -> Result<Vec<u8>, Error> {
    decode_with_output_len(input, src, (input.len() + src.len()) as u32 * 2)
}

pub fn decode_with_output_len(
    input: &[u8],
    src: &[u8],
    output_buffer_len: u32,
) -> Result<Vec<u8>, Error> {
    let input_len = input.len() as c_uint;
    let src_len = src.len() as c_uint;
    let mut avail_output = 0 as c_uint;
    let mut output = Vec::with_capacity(output_buffer_len as usize);
    let error_code = unsafe {
        binding::xd3_decode_memory(
            input.as_ptr(),
            input_len,
            src.as_ptr(),
            src_len,
            output.as_mut_ptr(),
            &mut avail_output,
            output_buffer_len,
            0,
        )
    };
    if error_code == 0 {
        // Extra sanity check to prevent UB.
        if avail_output > output_buffer_len {
            return Err(Error::OutOfBounds {
                expected_length: output_buffer_len,
                actual_length: avail_output,
            });
        }
        unsafe {
            output.set_len(avail_output as usize);
        }
        Ok(output)
    } else if error_code == libc::ENOSPC {
        Err(Error::InsufficientOutputLength)
    } else {
        Err(Error::XDelta3 { error_code })
    }
}
