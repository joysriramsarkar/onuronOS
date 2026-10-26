// shell/src/lib.rs — library surface for reusable shell modules.
//
// The lockscreen app (and other native consumers) reuse the device PIN
// verification logic instead of re-implementing (and re-breaking) it.
pub mod pin;
