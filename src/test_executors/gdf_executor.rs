use guid_create::GUID;
use log::debug;
use reqwest;
use std::sync::mpsc;

use crate::errors::{new_service_call_error, ErrorKind, Result};
use crate::gdf::{
    call_dialogflow, file_to_gdf_credentials, get_google_api_token, prepare_dialogflow_request,
    GDFCredentials, GoogleApisOauthToken,
};
use crate::json_parser::JsonParser;
use crate::yaml_parser::{Test, TestAssertion, TestAssertionResult, TestResult};

use crate::test_executors::TestExecutor;

pub type HttpClient = reqwest::blocking::Client;

pub struct GDFDefaultTestExecutor {
    test: Test,
    next_assertion: usize,
    http_client: HttpClient,
    token: GoogleApisOauthToken,
    conv_id: String,
    cred: GDFCredentials,
    tx: mpsc::Sender<Test>,
}

impl GDFDefaultTestExecutor {
    pub fn new(
        credentials_file: String,
        test: Test,
        tx: mpsc::Sender<Test>,
        http_proxy: Option<String>,
    ) -> Result<Self> {
        let http_client;

        match http_proxy {
            Some(proxy) => {
                debug!("building http client with proxy {}", proxy);
                http_client = HttpClient::builder()
                    .proxy(reqwest::Proxy::http(&proxy)?)
                    .build()?;
            }
            _ => {
                debug!("building http client with no proxy");
                http_client = HttpClient::new()
            }
        }

        let token = get_google_api_token(&credentials_file, &http_client)?;
        let conv_id = GUID::rand().to_string();
        let cred = file_to_gdf_credentials(&credentials_file)?;

        Ok(GDFDefaultTestExecutor {
            test,
            next_assertion: 0,
            http_client,
            token,
            conv_id,
            cred,
            tx,
        })
    }

    fn make_pretty_json(response: String) -> Result<String> {
        let val_orig: serde_json::Value = serde_json::from_str(&response)?;
        let changed_response = serde_json::to_string_pretty(&val_orig)?;
        Ok(changed_response)
    }
}

impl TestExecutor for GDFDefaultTestExecutor {
    fn move_to_next_assertion(&mut self) {
        self.next_assertion = self.next_assertion + 1;
    }

    fn move_behind_last_assertion(&mut self) {
        self.next_assertion = self.get_assertions().len() + 1;
    }

    fn get_assertions(&self) -> &Vec<TestAssertion> {
        &self.test.assertions
    }

    fn set_test_result(&mut self, test_result: TestResult) {
        self.test.test_result = Some(test_result);
    }

    fn set_test_assertion_result(&mut self, test_assertion_result: TestAssertionResult) {
        let idx = self.get_next_assertion_no();
        self.test.assertions[idx].test_assertion_result = Some(test_assertion_result);
    }

    fn get_next_assertion_no(&self) -> usize {
        self.next_assertion
    }

    fn send_test_results(&self) -> Result<()> {
        self.tx.send(self.test.clone())?;
        Ok(())
    }

    fn invoke_nlp(&self, assertion: &TestAssertion) -> Result<String> {
        let payload = prepare_dialogflow_request(&assertion.user_says, &self.test.lang);
        let resp = call_dialogflow(
            payload,
            &self.cred.project_id,
            &self.conv_id,
            &self.http_client,
            &self.token.access_token,
        )?;
        let resp = GDFDefaultTestExecutor::make_pretty_json(resp)?; // GDF sends pretty jsons but just for any case let's prettify it anyway
        let parser = JsonParser::new(&resp);
        let real_intent_name = parser.search("queryResult.intent.displayName")?;
        let real_intent_name = JsonParser::extract_as_string(&real_intent_name);

        if let Some(intent_name) = real_intent_name {
            if !assertion
                .bot_responds_with
                .contains(&intent_name.to_string())
            {
                let error_message = format!(
                    "Wrong intent name received. Expected one of: '{}', got: '{}'",
                    assertion.bot_responds_with.join(","),
                    intent_name
                );
                return Err(new_service_call_error(
                    ErrorKind::InvalidTestAssertionEvaluation,
                    error_message,
                    None,
                    Some(resp.to_owned()),
                ));
            }
        } else {
            let error_message = format!(
                "No intent name received. Expected: '{}'",
                assertion.bot_responds_with.join(",")
            );
            return Err(new_service_call_error(
                ErrorKind::InvalidTestAssertionEvaluation,
                error_message,
                None,
                Some(resp.to_owned()),
            ));
        }
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suite_executor::TestSuiteExecutor;
    use crate::thread_pool::ThreadPool;
    use crate::yaml_parser::TestSuite;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use yaml_rust::{Yaml, YamlLoader};

    // cargo test -- --show-output test_process_test
    #[test]
    #[ignore]
    fn test_process_test() -> Result<()> {
        const YAML_STR: &str =
        "
        suite-spec:
            name: 'Dummy Tracking'
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
                - userSays: 'it is 1234567891'
                  botRespondsWith: ['Tracking|CS|3|ID valid|Gen']
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: 'equals'
                      value: 'express_track'
                    - expression: 'queryResult.parameters.tracking_id'
                      operator: 'equals'
                      value: '1234567891'
       ";

        let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
        let yaml: &Yaml = &docs[0];
        let suite: TestSuite = TestSuite::from_yaml(yaml).unwrap();

        let mut suite_executor = TestSuiteExecutor::new(suite)?;
        let test1_executor = &mut suite_executor.test_executors[0];

        loop {
            println!();
            let details_result = test1_executor.next_assertion_details();

            if let None = details_result {
                println!("all assertions processed!");
                test1_executor.set_test_result(TestResult::Ok);
                break; // all asertions were processed -> break
            }

            let user_says = &details_result.unwrap().user_says;

            print!("Saying {}", user_says);
            let assertion_exec_result = test1_executor.execute_next_assertion();

            if let Some(_) = assertion_exec_result {
                print!(" - ok!");
            } else {
                print!(" - ko!");
                break;
            }
        }

        Ok(())
    }

    // cargo test -- --show-output test_process_multiple_tests
    #[test]
    #[ignore]
    fn test_process_multiple_tests() -> Result<()> {
        const YAML_STR: &str =
        "
        suite-spec:
            name: 'Dummy Tracking'
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
            - name: 'Hello - track - entity parsing'
              desc: 'Very similar second test'
              assertions:
                - userSays: 'Hi'
                  botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
                - userSays: 'track a package please'
                  botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
                  responseChecks:
                    - expression: 'queryResult.allRequiredParamsPresent'
                      operator: 'equals'
                      value: true
                - userSays: 'it is 1234567891'
                  botRespondsWith: ['Tracking|CS|3|ID valid|Gen']
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: 'equals'
                      value: 'express_track'
                    - expression: 'queryResult.parameters.tracking_id'
                      operator: 'equals'
                      value: '1234567891'
       ";

        let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
        let yaml: &Yaml = &docs[0];
        let suite: TestSuite = TestSuite::from_yaml(yaml).unwrap();

        let suite_executor = TestSuiteExecutor::new(suite)?;

        let running = Arc::new(AtomicBool::new(true));
        let pool = ThreadPool::new(4, running); // for workers is good match for modern multi core PCs

        let res_count = suite_executor.test_executors.len();

        for mut test_executor in suite_executor.test_executors {
            pool.execute(move || {
                loop {
                    let assertion_exec_result = test_executor.execute_next_assertion();
                    if let None = assertion_exec_result {
                        break;
                    }
                }
                println!("pool.execute closure done");
            });
        }
        println!("workers initiated!");

        for _ in 0..res_count {
            let test_result = suite_executor.rx.recv().unwrap();
            println!("test result {:#?}", test_result);
        }

        Ok(())
    }
}
