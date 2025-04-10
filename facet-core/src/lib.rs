#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

// Opaque pointer utilities
mod opaque;
pub use opaque::*;

// Specialization utilities
pub mod spez;

// Core trait definitions
mod _trait;
pub use _trait::*;

// Const type Id
mod typeid;
pub use typeid::*;

// Type definitions
mod types;
#[allow(unused_imports)] // wtf clippy? we're re-exporting?
pub use types::*;
