macro_rules! info {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        println!("{} {} {}", "info".green().bold(), ":".dimmed(), format!($($arg)*));
    };
}

macro_rules! _warn {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        eprintln!("{} {} {}", "warning".yellow().bold(), ":".dimmed(), format!($($arg)*).yellow());
    };
}

macro_rules! error {
    ($($arg:tt)*) => {
        #[allow(unused_imports)]
        use colored::Colorize as _;
        eprintln!("{} {} {}", "error".red().bold(), ":".dimmed(), format!($($arg)*).red());
    };
}

pub(crate) use _warn as warn;
pub(crate) use error;
pub(crate) use info;
