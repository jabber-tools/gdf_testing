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
    fn next_assertion_details(&self) -> Option<&TestAssertion> {
        if self.next_assertion >= self.test.assertions.len() {
            None
        } else {
            let assertion_to_execute = &self.test.assertions[self.next_assertion];
            Some(assertion_to_execute)
        }
    }
    fn execute_next_assertion(&mut self) -> Option<Result<String>> {
        // println!("next_assertion={}",self.next_assertion);
        if self.next_assertion >= self.test.assertions.len() {
            self.next_assertion = self.next_assertion + 1;
            return None;
        } else {
            let assertion_to_execute = &self.test.assertions[self.next_assertion];

            let assertion_response = self.invoke_nlp(assertion_to_execute);

            if let Err(intent_mismatch_error) = assertion_response {
                // if intent name does not match expected value do not continue
                self.next_assertion = self.next_assertion + 1;
                return Some(Err(intent_mismatch_error));
            }

            // otherwise try to run assertion response checks
            let assertion_response = assertion_response.unwrap();

            for response_check in &assertion_to_execute.response_checks {
                let response_check_result = TestSuiteExecutor::process_assertion_response_check(response_check, &assertion_response);

                if let Err(some_response_check_error) = response_check_result {
                    self.next_assertion = self.next_assertion + 1;
                    return Some(Err(some_response_check_error));
                }
            } 
            
            self.next_assertion = self.next_assertion + 1;
            return Some(Ok(assertion_response));
        };
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
    #[ignore]
    fn test_process_test() -> Result<()> {

        const YAML_STR: &str =
        "
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            cred: 'DEPRECATED - WILL BE REMOVED AND REPLACED BY GENERIC KEY/VAL STRUCTURE!'
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
    
        let mut config_map = HashMap::new();
        config_map.insert(
            "credentials_file".to_string(),
            "/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/src/testdata/credentials.json".to_string()
        );

        let mut suite_executor = TestSuiteExecutor::new(&suite, config_map)?;
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
    