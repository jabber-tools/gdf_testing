use crate::yaml_parser::{
    Test
};

pub struct HtmlResultReporter;

impl HtmlResultReporter {
    pub fn report_test_results(tests: &Vec<Test>) {
        // TBD
        println!("{:?}", tests);
    }
}

