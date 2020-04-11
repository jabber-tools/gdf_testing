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
use crate::executor::{TestExecutor};

pub struct VAPTestExecutor<'a> {
    vap_access_token: String,
    vap_url: String,
    test: &'a Test,
    parent_suite: &'a TestSuite
}

impl<'a> VAPTestExecutor<'a> {
    pub fn new(vap_access_token: String, vap_url: String, test: &'a Test, parent_suite: &'a TestSuite) -> Result<Self> {
        Ok(VAPTestExecutor {
            vap_access_token,
            vap_url,
            test,
            parent_suite,
        })
    }
}

impl<'a> TestExecutor for VAPTestExecutor<'a> {
    
    fn next_assertion_details(&self) -> Option<&TestAssertion> {
        None
    }    

    fn execute_next_assertion(&mut self) -> Option<Result<String>> {
        None
    }

    fn invoke_nlp(&self, assertion: &TestAssertion) -> Result<String> {
        Ok("".to_owned())
    }

}