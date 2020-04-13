use std::collections::HashMap;
use guid_create::GUID;

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
pub type HttpClient = reqwest::blocking::Client;

pub struct VAPTestExecutor<'a> {
    vap_access_token: String,
    vap_url: String,
    test: &'a Test,
    parent_suite: &'a TestSuite,
    http_client: HttpClient,
    next_assertion: usize,
    conv_id: String
}

impl<'a> VAPTestExecutor<'a> {
    pub fn new(vap_access_token: String, vap_url: String, test: &'a Test, parent_suite: &'a TestSuite) -> Result<Self> {

        let http_client = HttpClient::new();
        let conv_id = GUID::rand().to_string();

        Ok(VAPTestExecutor {
            vap_access_token,
            vap_url,
            test,
            parent_suite,
            http_client,
            next_assertion: 0,
            conv_id
        })
    }
}

impl<'a> TestExecutor for VAPTestExecutor<'a> {
    
    fn move_to_next_assertion(&mut self) {
        self.next_assertion = self.next_assertion + 1;
    }

    fn get_assertions(&self) -> &Vec<TestAssertion> {
        &self.test.assertions
    }

    fn get_next_assertion_no(&self) -> usize {
        self.next_assertion
    }

    fn invoke_nlp(&self, assertion: &TestAssertion) -> Result<String> {
        Ok("".to_owned())
    }

}