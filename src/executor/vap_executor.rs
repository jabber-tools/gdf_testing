use std::collections::HashMap;

use crate::yaml_parser::{
    Test, 
    TestAssertion, 
    TestSuiteType, 
    TestSuite, 
    TestAssertionResponseCheckOperator,
    TestAssertionResponseCheckValue,
    TestAssertionResponseCheck
};
use crate::errors::{Result, ErrorKind, new_error, Error};
use crate::executor::{TestExecutor, AssertionExecutionContext};

pub struct VAPTestExecutor {
    vap_access_token: String
}

impl TestExecutor for VAPTestExecutor {
    
    fn new(config: HashMap<String, String>) -> Self {
        VAPTestExecutor {
            vap_access_token: config.get("vap_access_token").unwrap().to_string()
        }
    }

    fn process_test(test: &Test, parent_suite: &TestSuite, project_id: &str) -> Result<()> {
        Ok(())
    }

    fn process_assertion(context: &AssertionExecutionContext) -> Result<()> {
        Ok(())
    }

    fn process_assertion_response_check(response_check: &TestAssertionResponseCheck, response: &str) -> Result<()> {
        Ok(())
    }
}

/* impl Iterator for VAPTestExecutor {
    type Item = TestAssertion;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
} */