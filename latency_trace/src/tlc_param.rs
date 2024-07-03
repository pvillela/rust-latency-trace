//! Defines traits and impls to support generic [`crate::LatencyTrace`].

use thread_local_collect::tlm::{
    joined::{Control as ControlJ, Holder as HolderJ},
    probed::{Control as ControlP, Holder as HolderP},
};

use crate::lt_collect_g::{op, AccRawTrace, RawTrace};

#[doc(hidden)]
pub trait TlcParam {
    type Control;
}

#[doc(hidden)]
pub trait TlcBase {
    fn new() -> Self;
    fn with_data_mut<V>(&self, f: impl FnOnce(&mut RawTrace) -> V) -> V;
}

pub trait TlcJoined: TlcBase {
    fn take_tls(&self);
    fn take_acc(&self, replacement: AccRawTrace) -> AccRawTrace;
}

pub trait TlcProbed: TlcJoined {
    fn probe_tls(&self) -> AccRawTrace;
}

//==============
// Impl for Probed

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

impl TlcJoined for ControlP<RawTrace, AccRawTrace> {
    fn take_tls(&self) {
        ControlP::take_tls(self)
    }

    fn take_acc(&self, replacement: AccRawTrace) -> AccRawTrace {
        ControlP::take_acc(self, replacement)
    }
}

impl TlcProbed for ControlP<RawTrace, AccRawTrace> {
    fn probe_tls(&self) -> AccRawTrace {
        ControlP::probe_tls(self)
    }
}

#[derive(Clone)]
pub struct Probed;

impl TlcParam for Probed {
    type Control = ControlP<RawTrace, AccRawTrace>;
}

//==============
// Impl for Joined

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

impl TlcJoined for ControlJ<RawTrace, AccRawTrace> {
    fn take_tls(&self) {
        ControlJ::take_own_tl(self)
    }

    fn take_acc(&self, replacement: AccRawTrace) -> AccRawTrace {
        ControlJ::take_acc(self, replacement)
    }
}

#[derive(Clone)]
pub struct Joined;

impl TlcParam for Joined {
    type Control = ControlJ<RawTrace, AccRawTrace>;
}
