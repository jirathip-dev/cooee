//! The one place cooee prints (#203): clippy denies `println!` and `eprintln!` everywhere else; use [`say!`](crate::say)
//! and [`note!`](crate::note).

use std::fmt::Arguments;

/// Writes `args` and a newline to stdout; `newline: false` leaves the line open (a prompt).
#[expect(clippy::print_stdout, reason = "the one stdout writer")]
pub fn stdout(args: Arguments, newline: bool) {
    if newline {
        println!("{args}");
    } else {
        print!("{args}");
    }
}

/// Writes `args` and a newline to stderr.
#[expect(clippy::print_stderr, reason = "the one stderr writer")]
pub fn stderr(args: Arguments) {
    eprintln!("{args}");
}

/// `println!` through [`stdout`].
#[macro_export]
macro_rules! say {
    () => { $crate::out::stdout(format_args!(""), true) };
    ($($t:tt)*) => { $crate::out::stdout(format_args!($($t)*), true) };
}

/// `print!` through [`stdout`].
#[macro_export]
macro_rules! say_open {
    ($($t:tt)*) => { $crate::out::stdout(format_args!($($t)*), false) };
}

/// `eprintln!` through [`stderr`].
#[macro_export]
macro_rules! note {
    () => { $crate::out::stderr(format_args!("")) };
    ($($t:tt)*) => { $crate::out::stderr(format_args!($($t)*)) };
}
