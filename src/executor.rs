use crate::errors::{Result, ErrorKind, new_error_from, Error};
use crate::yaml_parser::{Test, TestAssertion, TestSuiteType, TestSuite};
use crate::gdf::{get_google_api_token, prepare_dialogflow_request, call_dialogflow, file_to_gdf_credentials};
use reqwest;
use guid_create::GUID;
use yaml_rust::{YamlLoader, Yaml};

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
    
    for assertion in &test.assertions {
        let context = AssertionExecutionContext::new_context(assertion, &parent_suite.suite_spec.suite_type, &http_client, &conv_id, project_id, &token.access_token);
        process_assertion(&context);
    }
    Ok(())
}

pub fn process_assertion(context: &AssertionExecutionContext) -> Result<()> {
    match context.suite_type {
        TestSuiteType::DHLVAP => invoke_vap(context),
        TestSuiteType::DialogFlow => invoke_gdf(context),
    }?;
    Ok(())
}

pub fn invoke_gdf(context: &AssertionExecutionContext) -> Result<()> {
    println!("calling Dialogflow with utterance '{}'", context.assertion.user_says);

    let payload = prepare_dialogflow_request(context.assertion.user_says);
    let resp = call_dialogflow(payload, context.project_id, context.conv_id, context.http_client , context.bearer)?;
    // println!("{}", resp);
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
            - name: 'Welcome intent test'
              desc: 'Tests default welcome intent'
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: ['Welcome']
        ";     
        
        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];
        
        let suite =  TestSuite::from_yaml(yaml)?;
        let cred = file_to_gdf_credentials(suite.suite_spec.cred)?;
        process_test(&suite.tests[0], &suite, &cred.project_id)?;
        Ok(())
    }
}
    