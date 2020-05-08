use crate::errors::Result;
use crate::yaml_parser::Test;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct JsonResultReporter;

impl JsonResultReporter {
    pub fn report_test_results(tests: &Vec<Test>, file_path: &Path) -> Result<()> {
        let tests_json = serde_json::to_string_pretty(tests)?;
        let mut file = File::create(file_path)?;
        file.write_all(tests_json.as_bytes())?;
        Ok(())
    }
}
