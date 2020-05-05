use crate::yaml_parser::{
    Test
};

pub struct JsonResultReporter;

impl JsonResultReporter {
    pub fn report_test_results(tests: &Vec<Test>) {
        // TBD
        println!("{:?}", tests);
    }    
}

