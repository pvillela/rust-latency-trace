//! Defines traits and impls to support generic [`crate::lt_collect_g::LatencyTraceG`] parameterized by
//! `Control` objects from different [`thread_local_collect::tlm`] sub-modules.
//!
//! These traits are used internally only but have to be public because they are used in benchmarks
//! involving [`crate::LatencyTraceJ`] (which is hidden from generated documentation).

use crate::lt_collect_g::{op, AccRawTrace, RawTrace};
use thread_local_collect::tlm::{
    joined::{Control as ControlJ, Holder as HolderJ},
    probed::{Control as ControlP, Holder as HolderP},
};

pub trait TlcParam {
    type Control;
}

pub trait TlcBase {
    fn new() -> Self;
    fn with_data_mut<V>(&self, f: impl FnOnce(&mut RawTrace) -> V) -> V;
}

pub trait TlcDirect: TlcBase {
    fn take_tls(&self);
    fn take_acc(&self, replacement: AccRawTrace) -> AccRawTrace;
}

//==============
// Impl for Probed

#[derive(Clone)]
pub(crate) struct Probed;

impl TlcParam for Probed {
    type Control = ControlP<RawTrace, AccRawTrace>;
}

thread_local! {
    static LOCAL_INFO_PROBED: HolderP<RawTrace, AccRawTrace> = HolderP::new();
}

impl TlcBase for ControlP<RawTrace, AccRawTrace> {
    fn new() -> Self {
        ControlP::new(&LOCAL_INFO_PROBED, AccRawTrace::new(), RawTrace::new, op)
    }

    fn with_data_mut<V>(&self, f: impl FnOnce(&mut RawTrace) -> V) -> V {
        ControlP::with_data_mut(self, f)
    }
}

impl TlcDirect for ControlP<RawTrace, AccRawTrace> {
    fn take_tls(&self) {
        ControlP::take_tls(self)
    }

    fn take_acc(&self, replacement: AccRawTrace) -> AccRawTrace {
        ControlP::take_acc(self, replacement)
    }
}

//==============
// Impl for Joined

#[derive(Clone)]
pub struct Joined;

impl TlcParam for Joined {
    type Control = ControlJ<RawTrace, AccRawTrace>;
}

thread_local! {
    static LOCAL_INFO_JOINED: HolderJ<RawTrace, AccRawTrace> = HolderJ::new();
}

impl TlcBase for ControlJ<RawTrace, AccRawTrace> {
    fn new() -> Self {
        ControlJ::new(&LOCAL_INFO_JOINED, AccRawTrace::new(), RawTrace::new, op)
    }

    fn with_data_mut<V>(&self, f: impl FnOnce(&mut RawTrace) -> V) -> V {
        ControlJ::with_data_mut(self, f)
    }
}

impl TlcDirect for ControlJ<RawTrace, AccRawTrace> {
    fn take_tls(&self) {
        ControlJ::take_own_tl(self)
    }

    fn take_acc(&self, replacement: AccRawTrace) -> AccRawTrace {
        ControlJ::take_acc(self, replacement)
    }
}
