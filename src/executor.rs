use reqwest;
use guid_create::GUID;
use yaml_rust::{YamlLoader, Yaml};

use crate::errors::{Result, ErrorKind, new_error, Error};
use crate::json_parser::{JsonParser};
use crate::yaml_parser::{Test, TestAssertion, TestSuiteType, TestSuite};
use crate::gdf::{get_google_api_token, prepare_dialogflow_request, call_dialogflow, file_to_gdf_credentials};

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

pub fn process_test(test: &Test, parent_suite: &TestSuite, project_id: &str) -> Result<()> {
    let http_client = reqwest::blocking::Client::new();
    let token = get_google_api_token(parent_suite.suite_spec.cred, &http_client)?;
    let conv_id = GUID::rand().to_string();
    
    println!("");
    for assertion in &test.assertions {
        let context = AssertionExecutionContext::new_context(assertion, &parent_suite.suite_spec.suite_type, &http_client, &conv_id, project_id, &token.access_token);
        process_assertion(&context)?;
    }
    Ok(())
}

pub fn process_assertion(context: &AssertionExecutionContext) -> Result<()> {
    print!("saying '{}'", context.assertion.user_says);
    match context.suite_type {
        TestSuiteType::DHLVAP => invoke_vap(context),
        TestSuiteType::DialogFlow => invoke_gdf(context),
    }?;
    println!(" - ok!");
    Ok(())
}

pub fn invoke_gdf(context: &AssertionExecutionContext) -> Result<()> {
    // println!("calling Dialogflow with utterance '{}'", context.assertion.user_says);

    let payload = prepare_dialogflow_request(context.assertion.user_says);
    let resp = call_dialogflow(payload, context.project_id, context.conv_id, context.http_client , context.bearer)?;
    let parser = JsonParser::new(&resp);
    let realIntentName = parser.search("queryResult.intent.displayName")?;
    let realIntentName = JsonParser::extract_as_string(&realIntentName);

    if let Some(intentName) = realIntentName {
        if !context.assertion.bot_responds_with.contains(&intentName) {
            let error_message = format!("Wrong intent name received. Expected one of: '{}', got: '{}'", context.assertion.bot_responds_with.join(","), intentName);
            return Err(new_error(ErrorKind::InvalidTestAssertionEvaluation, error_message, None));
        }
    } else {
        let error_message = format!("No intent name received. Expected: '{}'", context.assertion.bot_responds_with.join(","));
        return Err(new_error(ErrorKind::InvalidTestAssertionEvaluation, error_message, None));
    }
    Ok(())
}

pub fn invoke_vap(context: &AssertionExecutionContext) -> Result<()> {
    // TBD...
    println!("invoking VAP {}", context.assertion.user_says);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // cargo test -- --show-output test_process_test
    #[test]
    // #[ignore]
    fn test_process_test() -> Result<()> {
        const YAML: &str =
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
                  botRespondsWith: ['Tracking|CS|0|Prompt|Gen2']
        ";     
        
        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];
        
        let suite =  TestSuite::from_yaml(yaml)?;
        let cred = file_to_gdf_credentials(suite.suite_spec.cred)?;
        let test_result = process_test(&suite.tests[0], &suite, &cred.project_id);
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
    