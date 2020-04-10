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

pub struct VAPTestExecutor<'a> {
    vap_access_token: String,
    test: &'a Test,
    parent_suite: &'a TestSuite
}

impl<'a> VAPTestExecutor<'a> {
    pub fn new(vap_access_token: String, test: &'a Test, parent_suite: &'a TestSuite) -> Self {
        VAPTestExecutor {
            vap_access_token,
            test,
            parent_suite
        }
    }
}

impl<'a> TestExecutor for VAPTestExecutor<'a> {
    
    fn process_test(&self) -> Result<()> {
        Ok(())
    }

    fn process_assertion(&self, context: &AssertionExecutionContext) -> Result<String> {
        Ok("".to_owned())
    }
}

pub fn invoke_vap(context: &AssertionExecutionContext) -> Result<String> {
    // TBD...
    println!("invoking VAP {}", context.assertion.user_says);
    Ok("tbd...".to_owned())
}