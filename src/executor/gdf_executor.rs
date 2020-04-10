use reqwest;
use guid_create::GUID;
use yaml_rust::{YamlLoader, Yaml};
use std::collections::HashMap;

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

use crate::executor::{TestExecutor, TestSuiteExecutor, AssertionExecutionContext};

pub struct GDFDefaultTestExecutor<'a> {
    credentials_file: String,
    test: &'a Test,
    parent_suite: &'a TestSuite
}

impl<'a> GDFDefaultTestExecutor<'a> {
    pub fn new(credentials_file: String, test: &'a Test, parent_suite: &'a TestSuite) -> Self {
        GDFDefaultTestExecutor {
            credentials_file,
            test,
            parent_suite
        }
    }
}

impl<'a> TestExecutor for GDFDefaultTestExecutor<'a> {
    
    fn process_test(&self) -> Result<()> {
        let http_client = reqwest::blocking::Client::new();
        let token = get_google_api_token(&self.parent_suite.suite_spec.cred, &http_client)?;
        let conv_id = GUID::rand().to_string();
        
        let cred = file_to_gdf_credentials(&self.parent_suite.suite_spec.cred)?;

        println!("");
        for assertion in &self.test.assertions {
            let context = AssertionExecutionContext::new_context(assertion, &http_client, &conv_id, &cred.project_id, &token.access_token);
            &self.process_assertion(&context)?;
        }
        Ok(())        
    }

    fn process_assertion(&self, context: &AssertionExecutionContext) -> Result<String> {
        print!("saying '{}'", context.assertion.user_says);
        let resp = invoke_gdf(context)?;
        println!(" - ok!");
        Ok(resp)
    }
}

pub fn invoke_gdf(context: &AssertionExecutionContext) -> Result<String> {
    // println!("calling Dialogflow with utterance '{}'", context.assertion.user_says);

    let payload = prepare_dialogflow_request(&context.assertion.user_says);
    let resp = call_dialogflow(payload, &context.project_id, &context.conv_id, context.http_client , &context.bearer)?;
    let parser = JsonParser::new(&resp);
    let realIntentName = parser.search("queryResult.intent.displayName")?;
    let realIntentName = JsonParser::extract_as_string(&realIntentName);

    if let Some(intentName) = realIntentName {
        if !context.assertion.bot_responds_with.contains(&intentName.to_string()) {
            let error_message = format!("Wrong intent name received. Expected one of: '{}', got: '{}'", context.assertion.bot_responds_with.join(","), intentName);
            return Err(new_error(ErrorKind::InvalidTestAssertionEvaluation, error_message, None));
        }
    } else {
        let error_message = format!("No intent name received. Expected: '{}'", context.assertion.bot_responds_with.join(","));
        return Err(new_error(ErrorKind::InvalidTestAssertionEvaluation, error_message, None));
    }
    Ok(resp)
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
            cred: '/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/src/testdata/credentials.json'
        tests:
            - name: 'Hello - track'
              desc: 'Simple initial two turn tracking dialog'
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
                - userSays: 'track a package'
                  botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
       ";           

        let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
        let yaml: &Yaml = &docs[0];
        let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    
    
        let mut config_map = HashMap::new();
        config_map.insert(
            "credentials_file".to_string(),
            suite.suite_spec.cred.to_owned()
        );

        let executor = TestSuiteExecutor::new(&suite, config_map);        

        let test_result = executor.test_executors[0].process_test();
        if let Err(err) =  test_result {
            match *err.kind {
                ErrorKind::InvalidTestAssertionEvaluation => {
                    println!(" - ko! {}", err.message);
                },
                _ =>  println!(" - ko! {}", err)
            }
        }
        Ok(())
    }
}
    