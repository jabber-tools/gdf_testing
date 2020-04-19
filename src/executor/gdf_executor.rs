use reqwest;
use guid_create::GUID;
use yaml_rust::{YamlLoader, Yaml};
use std::collections::HashMap;

use crate::errors::{Result, ErrorKind, new_service_call_error, new_error_from, Error};
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
    file_to_gdf_credentials,
    GoogleApisOauthToken,
    GDFCredentials
};

use crate::executor::{TestExecutor, TestSuiteExecutor};

pub type HttpClient = reqwest::blocking::Client;

pub struct GDFDefaultTestExecutor<'a> {
    test: &'a Test,
    parent_suite: &'a TestSuite,
    next_assertion: usize,
    http_client: HttpClient,
    token: GoogleApisOauthToken,
    conv_id: String,
    cred: GDFCredentials,
}

impl<'a> GDFDefaultTestExecutor<'a> {
    pub fn new(credentials_file: String, test: &'a Test, parent_suite: &'a TestSuite) -> Result<Self> {

        let http_client = HttpClient::new();
        let token = get_google_api_token(&credentials_file, &http_client)?;
        let conv_id = GUID::rand().to_string();
        let cred = file_to_gdf_credentials(&credentials_file)?;

        Ok(GDFDefaultTestExecutor {
            test,
            parent_suite,
            next_assertion: 0,
            http_client: http_client,
            token: token,
            conv_id: conv_id,
            cred: cred,
        })
    }
}

impl<'a> TestExecutor for GDFDefaultTestExecutor<'a> {

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

        let payload = prepare_dialogflow_request(&assertion.user_says);
        let resp = call_dialogflow(payload, &self.cred.project_id, &self.conv_id, &self.http_client, &self.token.access_token)?;
        let parser = JsonParser::new(&resp);
        let realIntentName = parser.search("queryResult.intent.displayName")?;
        let realIntentName = JsonParser::extract_as_string(&realIntentName);
    
        if let Some(intentName) = realIntentName {
            if !assertion.bot_responds_with.contains(&intentName.to_string()) {
                let error_message = format!("Wrong intent name received. Expected one of: '{}', got: '{}'", assertion.bot_responds_with.join(","), intentName);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionEvaluation, error_message, None, Some(resp.to_owned())));
            }
        } else {
            let error_message = format!("No intent name received. Expected: '{}'", assertion.bot_responds_with.join(","));
            return Err(new_service_call_error(ErrorKind::InvalidTestAssertionEvaluation, error_message, None, Some(resp.to_owned())));
        }
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
    // cargo test -- --show-output test_process_test
    #[test]
    // #[ignore]
    fn test_process_test() -> Result<()> {

        const YAML_STR: &str =
        "
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/src/testdata/credentials.json'
        tests:
            - name: 'Hello - track'
              desc: 'Simple initial two turn tracking dialog'
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
                - userSays: 'track a package'
                  botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
                  responseChecks:
                    - expression: 'queryResult.allRequiredParamsPresent'
                      operator: 'equals'
                      value: true
       ";           

        let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
        let yaml: &Yaml = &docs[0];
        let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    
    
        let mut suite_executor = TestSuiteExecutor::new(&suite)?;
        let test1_executor = &mut suite_executor.test_executors[0];

        while true {
            println!();
            let details_result = test1_executor.next_assertion_details();

            if let None = details_result {
                println!("all assertions processed!");
                break; // all asertions were processed -> break
            }

            let user_says = &details_result.unwrap().user_says;

            print!("Saying {}", user_says);
            let assertion_result = test1_executor.execute_next_assertion().unwrap();

            if let Err(err) =  assertion_result {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionEvaluation => {
                        print!(" - ko! {}", err.message);
                    },
                    _ =>  print!(" - ko! {}", err)
                }
            } else {
                print!(" - ok!");
            }
        }        

        Ok(())
    }        
}
    