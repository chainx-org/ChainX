pub use frame_support::debug;

pub const RUNTIME_TARGET: &'static str = "runtime";

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::rlog::debug::error!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::rlog::debug::error!(target: "runtime", $($arg)+);
    )
}
#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::rlog::debug::warn!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::rlog::debug::warn!(target: "runtime", $($arg)+);
    )
}
#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::rlog::debug::info!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::rlog::debug::info!(target: "runtime", $($arg)+);
    )
}
#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::rlog::debug::debug!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::rlog::debug::debug!(target: "runtime", $($arg)+);
    )
}
#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::rlog::debug::trace!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::rlog::debug::trace!(target: "runtime", $($arg)+);
    )
}
