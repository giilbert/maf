macro_rules! info {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        println!("{}: {}", "INFO".green().bold(), format!($($arg)*).green());
    };
}

macro_rules! warn {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        eprintln!("{}: {}", "WARNING".yellow().bold(), format!($($arg)*).yellow());
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        eprintln!("{}: {}", "ERROR".red().bold(), format!($($arg)*).red());
    };
}

pub(crate) use error;
pub(crate) use info;
