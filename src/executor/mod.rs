use reqwest;
use guid_create::GUID;
use std::collections::HashMap;
use yaml_rust::{YamlLoader, Yaml};

use crate::errors::{Result, ErrorKind, new_error, Error};
use crate::json_parser::{
    JsonParser, 
    JmespathType
};
use crate::yaml_parser::{
    Test, 
    TestAssertion, 
    TestSuiteType, 
    TestSuite, 
    TestAssertionResponseCheckOperator,
    TestAssertionResponseCheckValue,
    TestAssertionResponseCheck
};
use crate::gdf::{
    get_google_api_token, 
    prepare_dialogflow_request,
    call_dialogflow, 
    file_to_gdf_credentials
};

pub struct AssertionExecutionContext<'a> {
    assertion: &'a TestAssertion<'a>, 
    suite_type: &'a TestSuiteType, 
    http_client: &'a reqwest::blocking::Client, 
    conv_id: &'a str,
    project_id: &'a str,
    bearer: &'a str,
}

impl<'a> AssertionExecutionContext<'a> {
    fn new_context(assertion: &'a TestAssertion, suite_type: &'a TestSuiteType, 
    http_client: &'a reqwest::blocking::Client, conv_id: &'a str, 
    project_id: &'a str, bearer: &'a str) -> Self {
        AssertionExecutionContext {
            assertion,
            suite_type,
            http_client,
            conv_id,
            project_id,
            bearer
        }
    }
}

mod gdf_executor;
mod vap_executor;

pub trait TestExecutor {
    fn new(config: HashMap<String, String>) -> Self;
    fn process_test(test: &Test, parent_suite: &TestSuite, project_id: &str) -> Result<()>;
    fn process_assertion(context: &AssertionExecutionContext) -> Result<()>;
    fn process_assertion_response_check(response_check: &TestAssertionResponseCheck, response: &str) -> Result<()>;
}