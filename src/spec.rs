use std::fs::metadata;
use std::io::Error as IOError;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::Missing;

pub enum OutputStatus {
    UpToDate,
    FileMissing,
    OutOfDate,
    CannotDetermine(IOError),
}

use OutputStatus::{CannotDetermine, FileMissing, OutOfDate, UpToDate};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TemplateDef {
    pub name: String,
    pub data: PathBuf,
    pub template: PathBuf,
    pub output: PathBuf,
}

fn get_mod_time(p: impl AsRef<Path>) -> Result<SystemTime, IOError> {
    metadata(p)?.modified()
}

impl TemplateDef {
    pub fn new<S, P>(name: S, data: P, template: P, output: P) -> Result<Self, Missing>
    where
        P: Into<PathBuf>,
        S: Into<String>,
    {
        let spec = Self::new_unchecked(name.into(), data.into(), template.into(), output.into());

        spec.validate_files()?;
        Ok(spec)
    }

    pub const fn new_unchecked(
        name: String,
        data: PathBuf,
        template: PathBuf,
        output: PathBuf,
    ) -> Self {
        Self {
            name,
            data,
            template,
            output,
        }
    }

    pub fn validate_files(&self) -> Result<(), Missing> {
        let data_exists = self.data.exists();
        let template_exists = self.template.exists();

        match (data_exists, template_exists) {
            (true, true) => Ok(()),
            (data, template) => {
                let mut missing = Vec::new();
                if template {
                    missing.push(format!("template file: {}", self.template.display()));
                }
                if data {
                    missing.push(format!("data file: {}", self.data.display()));
                }
                Err(missing.into())
            }
        }
    }

    pub fn should_build(&self) -> bool {
        if let UpToDate = self.up_to_date() {
            false
        } else {
            true
        }
    }

    pub fn up_to_date(&self) -> OutputStatus {
        if !self.output.exists() {
            return FileMissing;
        }

        let output_modified = match get_mod_time(&self.output) {
            Ok(t) => t,
            Err(e) => {
                return CannotDetermine(e);
            }
        };

        let data_modified = match get_mod_time(&self.data) {
            Ok(t) => t,
            Err(e) => {
                return CannotDetermine(e);
            }
        };

        let template_modified = match get_mod_time(&self.template) {
            Ok(t) => t,
            Err(e) => {
                return CannotDetermine(e);
            }
        };

        if output_modified < template_modified || output_modified < data_modified {
            OutOfDate
        } else {
            UpToDate
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json;

    #[test]
    fn deser_single() {
        let actual: TemplateDef = serde_json::from_value(serde_json::json!({
            "name": "example",
            "data": "example.json",
            "template": "example.hbs",
            "output": "example.rst"
        }))
        .unwrap();

        let expected = TemplateDef::new_unchecked(
            "example".into(),
            "example.json".into(),
            "example.hbs".into(),
            "example.rst".into(),
        );

        assert_eq!(actual, expected);
    }
}
