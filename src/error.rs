use std::fmt::{Display, Error as FmtError, Formatter};
use std::io::Error as IOError;

use clap::Error as ClapError;
use handlebars::{RenderError, TemplateRenderError};
use serde_json::Error as JSONError;

macro_rules! error_impl {
    ( $( $x:ident ),* ) => {
        pub enum Error {
            $(
                $x(Box<$x>),  // Boxed due to variant size differences
            )*
        }

        $(
            impl From<$x> for Error {
                fn from(e: $x) -> Self {
                    Error::$x(Box::new(e))
                }
            }
        )*

        impl Display for Error {
            fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), FmtError> {
                match self {
                    $(
                        Error::$x(err) => write!(f, "{}", err),
                    )*
                }
            }
        }
    };
}

pub struct Missing(Vec<String>);

impl From<Vec<String>> for Missing {
    fn from(v: Vec<String>) -> Self {
        Self(v)
    }
}

impl Display for Missing {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), FmtError> {
        for msg in &self.0 {
            writeln!(f, "missing file: {}", msg)?;
        }
        Ok(())
    }
}

error_impl!(
    IOError,
    RenderError,
    JSONError,
    TemplateRenderError,
    ClapError,
    Missing
);

pub type Result<T> = std::result::Result<T, Error>;
