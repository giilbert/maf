macro_rules! info {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        println!("{}: {}", "INFO".green().bold(), format!($($arg)*));
    };
}

macro_rules! _warn {
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

pub(crate) use _warn as warn;
pub(crate) use error;
pub(crate) use info;
