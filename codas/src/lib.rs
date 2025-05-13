#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#![doc = include_str!("../README.md")]
//! > _Note_: This documentation is auto-generated
//! > from the project's README.md file.
extern crate alloc;

pub mod codec;
#[cfg(any(feature = "langs", test))]
pub mod langs;
#[cfg(any(feature = "parse", test))]
pub mod parse;
pub mod stream;
pub mod types;
