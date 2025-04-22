use std::{fmt::Debug, io::Write, str::FromStr};

use colored::Colorize;

pub fn input_string_blocking<T, V>(prompt: impl Into<String>, transform: V) -> anyhow::Result<T>
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

pub async fn input_string<T, V>(prompt: impl Into<String>, validate: V) -> anyhow::Result<T>
where
    T: FromStr + Debug + Send + 'static,
    T::Err: std::fmt::Debug,
    V: Fn(T) -> anyhow::Result<T> + Send + 'static,
{
    let prompt = prompt.into();
    let result = tokio::task::spawn_blocking(|| input_string_blocking::<T, V>(prompt, validate));
    result.await?
}

macro_rules! input {
    (transform: $transform:expr, $($arg:tt)*) => {
        {
            #[allow(unused_imports)]
            use colored::Colorize as _;
            use crate::input::input_string;
            use anyhow::Context as _;

            input_string(format!($($arg)*), $transform)
                .await
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
                .await
                .context(format!("failed to read input: {}", format!($($arg)*)))?
        }
    };
}

pub(crate) use input;

use crate::pretty;
