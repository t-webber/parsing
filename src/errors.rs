#[macro_export]
macro_rules! function {
    () => {{
        const fn brother() {}
        fn type_name_of<T>(_: T) -> &'static str {
            core::any::type_name::<T>()
        }
        let name = type_name_of(brother);
        name.get(..name.len().checked_sub(9).unwrap_or_default())
            .unwrap_or_default()
    }};
}

pub fn get_code_color(color: &str) -> String {
    format!(
        "\x1b[38;5;{}m",
        match color {
            "d" | "dark" | "black" => 0,
            "r" | "red" | "r1" | "dimmedred" | "dr" => 1,
            "r2" | "red2" | "brightred" | "br" => 9,
            "g" | "green" => 2,
            "g2" | "green2" | "brightgreen" | "bg" => 10,
            "y" | "yellow" => 3,
            "y2" | "yellow2" | "brightyellow" | "by" => 11,
            "b" | "blue" => 4,
            "b2" | "blue2" | "brightblue" | "bb" => 12,
            "m" | "magenta" | "pink" => 5,
            "m2" | "magenta2" | "brightmagenta" | "bm" => 13,
            "c" | "cyan" => 6,
            "c2" | "cyan2" | "brightcyan" | "bc" => 14,
            "w" | "white" | "light" => 15,
            // "w" | "white" | "light" => 7,
            _ => return color.to_owned(),
        }
    )
}

#[macro_export]
macro_rules! color_fmt {
    ($color:literal, $($arg:tt)*) => {
        format!("{}{}\x1b[0m", $crate::errors::get_code_color($color), format_args!($($arg)*)) // module_path!(),
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $crate::color_fmt!("r", "[ERROR   at {}:{}:{} in {}]\n   {}", file!(), line!(), column!(), $crate::function!(), format_args!($($arg)*)) // module_path!(),
        }
    };
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            color_fmt!("y", "[WARNING at {}:{}:{} in {}] {}", file!(), line!(), column!(), function!(), format_args!($($arg)*)) // module_path!(),
        }
    };
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
                 color_fmt!("green", "[SUCCESS in {}] {}", function!(), format_args!($($arg)*)) // module_path!(),
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            color_fmt!("blue", "[INFO    at {}:{}:{} in {}] {}", file!(), line!(), column!(), function!(), format_args!($($arg)*)) // module_path!(),
        }
    };
}

pub type SResult<A> = Result<A, String>;

#[allow(dead_code)]
pub trait ToError<A> {
    fn cast_error(self) -> SResult<A>;
    fn expl_error(self, msg: &str) -> SResult<A>;
}

impl<A> ToError<A> for Option<A> {
    fn cast_error(self) -> SResult<A> {
        self.ok_or_else(|| error!("Value found to be None, but expected Some."))
    }
    fn expl_error(self, msg: &str) -> SResult<A> {
        self.ok_or_else(|| error!("{}", msg))
    }
}

#[allow(clippy::absolute_paths)]
impl<A, E: std::fmt::Debug> ToError<A> for Result<A, E> {
    fn cast_error(self) -> SResult<A> {
        self.map_err(|err| error!("{err:?}"))
    }
    fn expl_error(self, msg: &str) -> SResult<A> {
        self.map_err(|err| error!("{msg}\n{err:?}"))
    }
}
