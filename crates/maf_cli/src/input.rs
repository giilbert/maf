use std::{fmt::Debug, io::Write, str::FromStr};

use colored::Colorize;

/// Get a string input from user, optionally validating or transforming it using a closure.
///
/// ## Arguments
/// * `prompt` - The prompt to display to the user. A purple question mark will be prefixed and a
///   space will be appended.
/// * `transform` - A closure that takes the input and returns a `Result<T, anyhow::Result>`. This
///   function should return Ok(T) where T is the desired input type if the input is valid, or an
///   error (made with anyhow::bail or similar) if the input is invalid.
///
/// This function blocks until the user provides valid input.
pub fn input_string<T, V>(prompt: impl Into<String>, transform: V) -> anyhow::Result<T>
where
    T: FromStr + Debug,
    T::Err: std::fmt::Debug,
    V: Fn(T) -> anyhow::Result<T>,
{
    let prompt = prompt.into();

    loop {
        print!(
            "{question_mark} {prompt} ",
            question_mark = "?".bright_purple().bold(),
        );
        std::io::stdout().flush()?;

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        buf = buf.trim().to_string();

        let value =
            T::from_str(&mut buf).map_err(|e| anyhow::anyhow!("failed to parse input: {e:?}"))?;

        match (transform)(value) {
            Ok(value) => {
                return Ok::<T, anyhow::Error>(value);
            }
            Err(e) => {
                pretty::error!("{e:?} Try again.");
            }
        }
    }
}

/// A macro that wraps the `input_string` function to simplify input handling.
///
/// Prompts are formatted using `format!`, and an optional closure can be provided to transform
/// the input. If no closure is provided, inputs are considered valid as is.
///
/// ## Examples
///
/// ```rust
/// // A type is needed to specify what type to parse the input into:
/// let age: u32 = input!("Enter your age:");
/// // Specify a prompt with a transformation function:
/// let name: String = input!(transform: |s: String| Ok(s.trim().to_string()), "Enter your name:");
/// ```
macro_rules! input {
    (transform: $transform:expr, $($arg:tt)*) => {
        {
            #[allow(unused_imports)]
            use colored::Colorize as _;
            use crate::input::input_string;
            use anyhow::Context as _;

            input_string(format!($($arg)*), $transform)
                .context(format!("failed to read input: {}", format!($($arg)*)))?
        }
    };
    ($($arg:tt)*) => {
        {
            #[allow(unused_imports)]
            use colored::Colorize as _;
            use crate::input::input_string;
            use anyhow::Context as _;

            input_string(format!($($arg)*), |t| Ok(t))
                .context(format!("failed to read input: {}", format!($($arg)*)))?
        }
    };
}

pub(crate) use input;

use crate::pretty;
