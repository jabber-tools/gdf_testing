use serde::{Serialize, Deserialize};
use serde_json::json;
use guid_create::GUID;
use std::sync::mpsc;

use crate::yaml_parser::{
    Test, 
    TestResult,
    TestAssertion, 
    TestAssertionResult
};
use crate::json_parser::JsonParser;
use reqwest::header::{HeaderMap, HeaderValue};
use crate::errors::{Result, ErrorKind, new_service_call_error};
use crate::test_executors::TestExecutor;
pub type HttpClient = reqwest::blocking::Client;

#[derive(Debug, Serialize, Deserialize)]
pub struct VapAuthenticationResponseAuthentication {
    strategy: String
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct VapAuthenticationResponseUser {
    userId: String,
    email: String,
    description: String,
    allowedServices: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct VapAuthenticationResponse {
    pub accessToken: String,
    pub authentication:  VapAuthenticationResponseAuthentication,
    pub user: VapAuthenticationResponseUser
}

fn prepare_vap_request(vap_access_token: &str, utterance: &str, conv_id: &str, lang: &str) -> String {
    // so far we do not support neither vaContext dynamic enhancement nor development identity
    format!(r#"{{
        "headers": {{
            "at": "{_access_token_}",
            "content-type": "application/json"
        }},
        "body": {{
            "text": "{_utterance_}",
            "convId": "{_conv_id_}"
        }},
        "vaContext": {{
            "lang": "{_lang_}"
        }}
    }}"#, _access_token_ = vap_access_token, _utterance_ = utterance, _conv_id_ = conv_id, _lang_ = lang)
}

fn call_vap (payload: String, http_client: &HttpClient, bearer: &str, vap_url: &str) -> Result<String> {
    let mut headers = HeaderMap::new();
    let bearer_str = format!("{}", bearer);
    headers.insert("Authorization", HeaderValue::from_str(&bearer_str).unwrap());
    headers.insert("Content-Type", HeaderValue::from_str("application/json").unwrap());
    
    let vap_url = format!("{}/vapapi/channels/generic/v1",vap_url);
    let resp = http_client.post(&vap_url).body(payload).headers(headers).send()?.text()?;
    Ok(resp)
}

fn remove_va_context_config(response: String) -> Result<String>  {
    let mut val_orig: serde_json::Value = serde_json::from_str(&response)?;

    let dummy_config = json!({
        "note": "config removed for security reasons"
      });
      
    val_orig["vaContext"]["config"] = dummy_config;
    let changed_response = serde_json::to_string(&val_orig)?;

    Ok(changed_response)
}

pub struct VAPTestExecutor {
    vap_access_token: String,
    vap_url: String,
    test: Test,
    http_client: HttpClient,
    next_assertion: usize,
    conv_id: String,
    jwt_token: String,
    tx: mpsc::Sender<Test>
}

impl VAPTestExecutor {
    pub fn new(vap_access_token: String, vap_url: String, vap_svc_account_email: String, vap_svc_account_password: String, test: Test, tx: mpsc::Sender<Test>) -> Result<Self> {

        let http_client = HttpClient::new();
        let conv_id = GUID::rand().to_string();

        let jwt_token = VAPTestExecutor::get_vap_access_token(&vap_svc_account_email, &vap_svc_account_password, &vap_url)?.accessToken;

        Ok(VAPTestExecutor {
            vap_access_token,
            vap_url,
            test,
            http_client,
            next_assertion: 0,
            conv_id,
            jwt_token,
            tx
        })
    }

    fn get_vap_access_token(svc_account_email: &str, svc_account_password: &str, vap_url: &str) -> Result<VapAuthenticationResponse> {
        let body = format!(r#"{{
            "strategy": "local",
            "email": "{vap_svc_acc_email}",
            "password": "{vap_svc_acc_pwd}"
          }}"#, vap_svc_acc_email=svc_account_email, vap_svc_acc_pwd=svc_account_password);
        
        let url = format!("{}/vapapi/authentication/v1", vap_url);

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_str("application/json").unwrap());
        
        let http_client = HttpClient::new();

        let resp = http_client.post(&url).body(body).headers(headers).send()?.text()?;
        let vap_auth_resp = serde_json::from_str::<VapAuthenticationResponse>(&resp)?;

        /*        
        VAP authentication service will return something like this:
            {
                "accessToken": "eyJ...",
                "authentication": {
                    "strategy": "local"
                },
                "user": {
                    "userId": "666",
                    "email": "dummy-cs@iam.vap.dhl.com",
                    "description": "dummy account used for development and integration testing of GDF testing tool",
                    "allowedServices": [
                        "vapapi/channels/generic/v1"
                    ]
                }
            }        
        */
        Ok(vap_auth_resp)

    }
}

impl TestExecutor for VAPTestExecutor {
    
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

        let payload = prepare_vap_request(&self.vap_access_token, &assertion.user_says, &self.conv_id, &self.test.lang);
        let resp = call_vap(payload, &self.http_client, &self.jwt_token, &self.vap_url)?;
        let resp = remove_va_context_config(resp)?; // remove vaContext.config since it contains sensitive data
        let parser = JsonParser::new(&resp);
        let real_intent_name = parser.search("dfResponse.queryResult.intent.displayName")?;
        let real_intent_name = JsonParser::extract_as_string(&real_intent_name);
    
        if let Some(intent_name) = real_intent_name {
            if !assertion.bot_responds_with.contains(&intent_name.to_string()) {
                let error_message = format!("Wrong intent name received. Expected one of: '{}', got: '{}'", assertion.bot_responds_with.join(","), intent_name);
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
    use crate::thread_pool::ThreadPool;
    use yaml_rust::{YamlLoader, Yaml};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use crate::suite_executor::TestSuiteExecutor;
    use crate::yaml_parser::TestSuite;

    const YAML_STR: &str =
    "
    suite-spec:
        name: 'Dummy Tracking'
        type: 'DHLVAP'
        config: 
          - vap_url: 'https://vap-dev.prg-dc.dhl.com:7070'
          - vap_access_token: '00b2018c-1a78-415c-8999-0852d503b1f3'
          - vap_svc_account_email: 'dummy-cs@iam.vap.dhl.com'
          - vap_svc_account_password: 'dummyPassword123'
    tests:
        - name: 'Hello - track'
          desc: 'Simple initial two turn tracking dialog'
          assertions:
            - userSays: 'Hello'
              botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
            - userSays: 'track a package'
              botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
              responseChecks:
                - expression: 'dfResponse.queryResult.allRequiredParamsPresent'
                  operator: 'equals'
                  value: true
                - expression: 'vaContext.context.channelId'
                  operator: 'equals'
                  value: 'vap-generic'
   ";      
    
    #[test]
    fn test_get_vap_config() {

       let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
       let yaml: &Yaml = &docs[0];
       let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    
    
       assert_eq!(suite.suite_spec.config.get("vap_url").unwrap(), "https://vap-dev.prg-dc.dhl.com:7070");
       assert_eq!(suite.suite_spec.config.get("vap_access_token").unwrap(), "00b2018c-1a78-415c-8999-0852d503b1f3");
       assert_eq!(suite.suite_spec.config.get("vap_svc_account_email").unwrap(), "dummy-cs@iam.vap.dhl.com");
       assert_eq!(suite.suite_spec.config.get("vap_svc_account_password").unwrap(), "dummyPassword123");
        
    }    

    // cargo test -- --show-output test_get_vap_token
    #[test]
    #[ignore]
    fn test_get_vap_token() -> Result<()> {

       let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
       let yaml: &Yaml = &docs[0];
       let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    

       #[allow(unused_variables)]
       let (tx, rx) = mpsc::channel();

       let executor = VAPTestExecutor::new(
        suite.suite_spec.config.get("vap_access_token").unwrap().to_owned(),
        suite.suite_spec.config.get("vap_url").unwrap().to_owned(),
        suite.suite_spec.config.get("vap_svc_account_email").unwrap().to_owned(),
        suite.suite_spec.config.get("vap_svc_account_password").unwrap().to_owned(),
        suite.tests[0].clone(), tx).unwrap();
        
        assert_eq!(executor.jwt_token.trim().len() > 0, true);

        let vap_access_token = VAPTestExecutor::get_vap_access_token(
            suite.suite_spec.config.get("vap_svc_account_email").unwrap(), 
            suite.suite_spec.config.get("vap_svc_account_password").unwrap(), 
            suite.suite_spec.config.get("vap_url").unwrap()
        ).unwrap();
        println!("{:#?}",vap_access_token);

        assert_eq!(vap_access_token.authentication.strategy, "local");
        assert_eq!(vap_access_token.user.userId, "666");
        assert_eq!(vap_access_token.user.description, "dummy account used for development and integration testing of GDF testing tool");
        assert_eq!(vap_access_token.user.allowedServices.len(), 1);
        assert_eq!(vap_access_token.user.allowedServices[0], "vapapi/channels/generic/v1");

        Ok(())
    }

    // cargo test -- --show-output test_process_vap_test
    #[test]
    #[ignore]
    fn test_process_vap_test() -> Result<()> {

        let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
        let yaml: &Yaml = &docs[0];
        let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    
    
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

            if let Some(_) =  assertion_exec_result {
                print!(" - ok!");
            } else {
                print!(" - ko!");
                break;
            }
        }        

        Ok(())
    }    

    // cargo test -- --show-output test_process_vap_multiple_tests
    #[test]
    #[ignore]
    fn test_process_vap_multiple_tests() -> Result<()> {
        let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
        let yaml: &Yaml = &docs[0];
        let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    
    
        let suite_executor = TestSuiteExecutor::new(suite)?;

        let running = Arc::new(AtomicBool::new(true));
        let pool = ThreadPool::new(4, running); // for workers is good match for modern multi core PCs

        let res_count = suite_executor.test_executors.len();

        for mut test_executor in suite_executor.test_executors {
            pool.execute(move || {
        
                loop {
                    let assertion_exec_result = test_executor.execute_next_assertion();
                    if let None =  assertion_exec_result {
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