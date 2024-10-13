//! This wraps [`ByondValue`] with IncRef/DecRef and [`Rc`]
//!
//! [`ByondValue`] only lives for one tick, but [`crate::load::command_buffer::CommandBuffer`] needs to hold them for
//! longer than one tick, so we have to do this.

use byondapi::value::ByondValue;
use std::rc::Rc;

/// This type is used to wrap a ByondValue in IncRef/DecRef
#[derive(Debug)]
pub struct SmartByondValue {
    _internal: ByondValue,
}

impl From<ByondValue> for SmartByondValue {
    fn from(mut value: ByondValue) -> Self {
        value.increment_ref();
        SmartByondValue { _internal: value }
    }
}

impl Drop for SmartByondValue {
    fn drop(&mut self) {
        self._internal.decrement_ref()
    }
}

impl SmartByondValue {
    pub fn get_temp_ref(&self) -> ByondValue {
        self._internal
    }
}

/// For when you need to also share ownership of the smart ref.
pub type SharedByondValue = Rc<SmartByondValue>;
