mod cleanup;
#[cfg(unix)]
mod signal;
mod tee;
mod wait;

pub(crate) use wait::wait_with_optional_timeout;
