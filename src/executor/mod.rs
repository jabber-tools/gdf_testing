use std::sync::mpsc;

use crate::errors::{Result, ErrorKind, new_service_call_error, new_error, new_error_from};
use crate::json_parser::{
    JsonParser, 
    JmespathType
};
use crate::yaml_parser::{
    Test, 
    TestResult,
    TestAssertion, 
    TestAssertionResult,
    TestSuiteType, 
    TestSuite, 
    TestAssertionResponseCheckOperator,
    TestAssertionResponseCheckValue,
    TestAssertionResponseCheck
};

mod gdf_executor;
mod vap_executor;
use vap_executor::VAPTestExecutor;
use gdf_executor::GDFDefaultTestExecutor;

pub trait TestExecutor {
    // helper abstaract methods so that we can use default implementations for next_assertion_details/execute_next_assertion
    fn move_to_next_assertion(&mut self);
    fn move_behind_last_assertion(&mut self);
    fn get_assertions(&self) -> &Vec<TestAssertion>;
    fn set_test_result(&mut self, test_result: TestResult);
    fn set_test_assertion_result(&mut self, test_assertion_result: TestAssertionResult);
    fn get_next_assertion_no(&self) -> usize;
    fn send_test_results(&self) -> Result<()>;
    //
    // core abstract method to be provided for every test executor //
    //
    fn invoke_nlp(&self, assertion: &TestAssertion) -> Result<String>;

    // these default implementation hardcode default flow for convenience
    // every test executor can than focus on invoke_nlp only
    fn next_assertion_details(&self) -> Option<&TestAssertion> {
        let next_assertion_no = self.get_next_assertion_no();
        let assertions = self.get_assertions();

        if next_assertion_no >= assertions.len() {
            let _ = self.send_test_results();
            None
        } else {
            let assertion_to_execute = &assertions[next_assertion_no];
            Some(assertion_to_execute)
        }
    }

    fn execute_next_assertion(&mut self) -> Option<()> {

        let next_assertion_no = self.get_next_assertion_no();
        let assertions = self.get_assertions();

        if next_assertion_no >= assertions.len() {
            self.set_test_result(TestResult::Ok);
            let _ = self.send_test_results();
            return None;
        } else {
            let assertion_to_execute = &assertions[next_assertion_no];

            let assertion_response = self.invoke_nlp(assertion_to_execute);

            if let Err(intent_mismatch_error) = assertion_response {
                // if intent name does not match expected value do not continue
                self.set_test_assertion_result(TestAssertionResult::KoIntentNameMismatch(intent_mismatch_error));
                self.set_test_result(TestResult::Ko);
                self.move_behind_last_assertion();
                let _ = self.send_test_results();
                return None;
            } 

            // otherwise try to run assertion response checks
            let assertion_response = assertion_response.unwrap();

            for response_check in &assertion_to_execute.response_checks {
                let response_check_result = TestSuiteExecutor::process_assertion_response_check(response_check, &assertion_response);
    
                if let Err(some_response_check_error) = response_check_result {
                    self.set_test_assertion_result(TestAssertionResult::KoResponseCheckError(some_response_check_error));
                    self.set_test_result(TestResult::Ko);
                    self.move_behind_last_assertion();
                    let _ = self.send_test_results();
                    return None;
                }
            } 
            
            self.set_test_assertion_result(TestAssertionResult::Ok(assertion_response));
            self.move_to_next_assertion();
            return Some(());                

        }
    }      
}

pub struct TestSuiteExecutor<'a> {
    pub test_suite: TestSuite,
    pub test_executors: Vec<Box<dyn TestExecutor + 'a + Send>>, // Box references are by default 'static! we must ecplivitly indicate shorter lifetime
    pub rx: mpsc::Receiver<Test>
}

impl<'a> TestSuiteExecutor<'a> {
    pub fn new(test_suite: TestSuite) -> Result<Self> {
        
        let (tx, rx) = mpsc::channel(); // channel for receoving the results of tests which are running in parallel

        let mut test_executors:Vec<Box<dyn TestExecutor + 'a + Send>> = vec![];

        match test_suite.suite_spec.suite_type {
            TestSuiteType::DHLVAP => {

                let vap_access_token = test_suite.suite_spec.config.get("vap_access_token");
                if let None = vap_access_token {
                    return Err(new_error_from(ErrorKind::GenericError("vap_access_token config value not found".to_owned())));            
                }
                let vap_access_token = vap_access_token.unwrap();

                let vap_url = test_suite.suite_spec.config.get("vap_url");
                if let None = vap_url {
                    return Err(new_error_from(ErrorKind::GenericError("vap_url config value not found".to_owned())));            
                }
                let vap_url = vap_url.unwrap();

                let vap_svc_account_email = test_suite.suite_spec.config.get("vap_svc_account_email");
                if let None = vap_svc_account_email {
                    return Err(new_error_from(ErrorKind::GenericError("vap_svc_account_email config value not found".to_owned())));            
                }
                let vap_svc_account_email = vap_svc_account_email.unwrap();

                let vap_svc_account_password = test_suite.suite_spec.config.get("vap_svc_account_password");
                if let None = vap_svc_account_password {
                    return Err(new_error_from(ErrorKind::GenericError("vap_svc_account_password config value not found".to_owned())));            
                }
                let vap_svc_account_password = vap_svc_account_password.unwrap();

                for (idx, test) in test_suite.tests.iter().enumerate() {
                    let mut _test = test.clone();
                    _test.execution_id = Some(idx);

                    let _executor = Box::new(VAPTestExecutor::new(
                        vap_access_token.to_owned(),
                        vap_url.to_owned(),
                        vap_svc_account_email.to_owned(),
                        vap_svc_account_password.to_owned(),
                        _test, tx.clone())?) as Box<dyn TestExecutor + Send>;
                    test_executors.push(_executor);
                }

                Ok(TestSuiteExecutor {
                    test_suite,
                    test_executors,
                    rx
                })
            },
            TestSuiteType::DialogFlow => {

                let credentials_file = test_suite.suite_spec.config.get("credentials_file");
                if let None = credentials_file {
                    return Err(new_error_from(ErrorKind::GenericError("credentials_file config value not found".to_owned())));            
                }
                let credentials_file = credentials_file.unwrap();
        
                for (idx, test) in test_suite.tests.iter().enumerate() {
                    let mut _test = test.clone();
                    _test.execution_id = Some(idx);
                    let _executor = Box::new(GDFDefaultTestExecutor::new(credentials_file.to_owned(), _test, tx.clone())?) as Box<dyn TestExecutor + Send>;
                    test_executors.push(_executor);
                }

                Ok(TestSuiteExecutor {
                    test_suite,
                    test_executors,
                    rx
                })
            },
        }
    }


    pub fn process_assertion_response_check(response_check: &TestAssertionResponseCheck, response: &str) -> Result<()> {
        /****  HELPER closures ****/
        let process_bool_equals = |bool_val_expected: &bool| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;

            let value = JsonParser::extract_as_bool(&search_result);
            if let Some(bool_val_real) = value {
                if bool_val_real == *bool_val_expected {
                    return Ok(());
                } else {
                    let error_message = format!("Expected value ({}) does not match real value: ({}) for expression: {}", bool_val_expected, bool_val_real, response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            } else {
                let error_message = format!("Unable to retrieve boolean value ({}) for expression: {}", bool_val_expected, response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };

        let process_bool_not_equals = |bool_val_expected: &bool| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;
            let value = JsonParser::extract_as_bool(&search_result);
            if let Some(bool_val_real) = value {
                if bool_val_real != *bool_val_expected {
                    return Ok(());
                } else {
                    let error_message = format!("Expected value ({}), got instead value: ({}) for expression: {}", !bool_val_expected, bool_val_real, response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            } else {
                let error_message = format!("Unable to retrieve boolean value ({}) for expression: {}", !bool_val_expected, response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };

        let process_string_equals = |str_val_expected: &String| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;
            let value = JsonParser::extract_as_string(&search_result);
            if let Some(str_val_real) = value {
                if str_val_real == str_val_expected {
                    return Ok(());
                } else {
                    let error_message = format!("Expected value '{}' does not match real value: '{}' for expression: {}", str_val_expected, str_val_real, response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            } else {
                let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };

        let process_string_includes = |str_val_expected: &String| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;
            let value = JsonParser::extract_as_string(&search_result);
            if let Some(str_val_real) = value {
                if str_val_real.contains(str_val_expected) == true {
                    return Ok(());
                } else {
                    let error_message = format!("Expected value '{}' not included in real value: '{}' for expression: {}", str_val_expected, str_val_real, response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            } else {
                let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };

        let process_string_json_equals = |str_val_expected: &String| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;

            if JsonParser::get_jmespath_var_type(&search_result) == Some(JmespathType::Array) {
               
                let value = JsonParser::extract_as_array(&search_result);
                if let Some(array_val_real) = value {
                    let json_comparison_result = JsonParser::compare_array_with_str(&array_val_real, &str_val_expected);

                    match json_comparison_result {
                        Ok(str_val) if str_val == "__OK__" => return Ok(()),
                        Ok(err_msg) => {
                            let error_message = format!("Arrays not matching for expression '{}'. Error: {}", response_check.expression, err_msg);
                            return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                        },
                        Err(error) => {
                            let error_message = format!("Arrays not matching for expression '{}'. Error: {}", response_check.expression, error);
                            return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                        }
                    }
                } else {
                    let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            
            } else if JsonParser::get_jmespath_var_type(&search_result) == Some(JmespathType::Object) {
                let value = JsonParser::extract_as_object(&search_result);

                if let Some(obj_val_real) = value {
                    let json_comparison_result = JsonParser::compare_object_with_str(&obj_val_real, &str_val_expected);

                    match json_comparison_result {
                        Ok(str_val) if str_val == "__OK__" => return Ok(()),
                        Ok(err_msg) => {
                            let error_message = format!("Objects not matching for expression '{}'. Error: {}", response_check.expression, err_msg);
                            return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                        },
                        Err(error) => {
                            let error_message = format!("Objects not matching for expression '{}'. Error: {}", response_check.expression, error);
                            return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                        }
                    }
                } else {
                    let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
                
            } else {
                let error_message = format!("Cannot apply jsonequals operator. Retrieved value is neither object nor array for expression: {}", response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };

        let process_string_not_equals = |str_val_expected: &String| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;
            let value = JsonParser::extract_as_string(&search_result);
            if let Some(str_val_real) = value {
                if str_val_real != str_val_expected {
                    return Ok(());
                } else {
                    let error_message = format!("Expected value '{}' does match real value: '{}' for expression: {}", str_val_expected, str_val_real, response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            } else {
                let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };

        let process_num_equals = |num_val_expected: &f64| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;
            let value = JsonParser::extract_as_number(&search_result);
            if let Some(num_val_real) = value {
                if num_val_real == *num_val_expected {
                    return Ok(());
                } else {
                    let error_message = format!("Expected value ({}) does not match real value: ({}) for expression: {}", num_val_expected, num_val_real, response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            } else {
                let error_message = format!("Unable to retrieve numerical value for expression: {}", response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };

        let process_num_length = |num_val_expected: &f64| -> Result<()> {
                        // we do support length of arrays only, not lenght of strings or number of digits in number!
                        let parser = JsonParser::new(response);
                        let search_result = parser.search(&response_check.expression)?;
    
                        match JsonParser::get_jmespath_var_type(&search_result) {
                            Some(JmespathType::Array) => /* array type */ {
                                let value = JsonParser::extract_as_array(&search_result);
                                if let Some(arr_value) = value {
                                    if arr_value.len() == *num_val_expected as usize { // TODO: num value in response check should be usize, f64 does not make sense if used only for array length comparison
                                        return Ok(());
                                    } else {
                                        let error_message = format!("Expected array length {}, got {} for expression: {}", num_val_expected, arr_value.len(), response_check.expression);
                                        return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                                    }
                                } else {
                                    let error_message = format!("Unable to retrieve array value for expression: {}", response_check.expression);
                                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                                }
        
                            },
                            /* no type, i.e. expression does not match any value in json */
                            Some(JmespathType::Null) |
                            None =>  {
                                let error_message = format!("Unable to retrieve array value for expression: {}", response_check.expression);
                                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                            },
                            Some(_) => /* some other type, e.g. object*/ {
                                let error_message = format!("Operator length allowed for array expressions only. Expression: {}", response_check.expression);
                                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                            }
                        }
        };
        
        let process_num_not_equals = |num_val_expected: &f64| -> Result<()> {
            let parser = JsonParser::new(response);
            let search_result = parser.search(&response_check.expression)?;
            let value = JsonParser::extract_as_number(&search_result);
            if let Some(num_val_real) = value {
                if num_val_real != *num_val_expected {
                    return Ok(());
                } else {
                    let error_message = format!("Expected value not equal to ({}) got value: ({}) for expression: {}", num_val_expected, num_val_real, response_check.expression);
                    return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
                }
            } else {
                let error_message = format!("Unable to retrieve numerical value for expression: {}", response_check.expression);
                return Err(new_service_call_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None, Some(response.to_owned())));
            }
        };        

        /**** MAIN processing logic of the function utilizing closures defined above ****/
        match &response_check.value {
        
            TestAssertionResponseCheckValue::BoolVal(bool_val_expected) => {
                
                match response_check.operator {
                    TestAssertionResponseCheckOperator::Equals => return process_bool_equals(bool_val_expected),
                    TestAssertionResponseCheckOperator::Includes => {
                        let error_message = format!("Operator includes not allowed for boolean value of expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    },
                    TestAssertionResponseCheckOperator::JsonEquals => {
                        let error_message = format!("Operator jsonequals not allowed for boolean value of expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    },
                    TestAssertionResponseCheckOperator::Length => {
                        let error_message = format!("Operator length not allowed for boolean value of expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    },
                    TestAssertionResponseCheckOperator::NotEquals => return process_bool_not_equals(bool_val_expected)
                }
    
            },
    
            TestAssertionResponseCheckValue::StrVal(str_val_expected) => {
                match response_check.operator {
                    TestAssertionResponseCheckOperator::Equals => return process_string_equals(str_val_expected),
                    TestAssertionResponseCheckOperator::Includes => return process_string_includes(str_val_expected),
                    TestAssertionResponseCheckOperator::JsonEquals => return process_string_json_equals(str_val_expected),
                    TestAssertionResponseCheckOperator::Length => {
                        let error_message = format!("Operator length not allowed for string value of expression: '{}'. If value is '4' use 4 instead.", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    },
                    TestAssertionResponseCheckOperator::NotEquals => return process_string_not_equals(str_val_expected)
                }
    
            },
    
            TestAssertionResponseCheckValue::NumVal(num_val_expected) => {
                match response_check.operator {
                    TestAssertionResponseCheckOperator::Equals => return process_num_equals(num_val_expected),
                    TestAssertionResponseCheckOperator::Includes => {
                        let error_message = format!("Operator includes not allowed for numeric value of expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    },
                    TestAssertionResponseCheckOperator::JsonEquals => {
                        let error_message = format!("Operator jsonequals not allowed for numeric value of expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    },
                    TestAssertionResponseCheckOperator::Length => return process_num_length(num_val_expected),
                    TestAssertionResponseCheckOperator::NotEquals => return process_num_not_equals(num_val_expected)
                }
            }
        }
    }    
} 

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str =
    r#"
    {
        "responseId": "24f4edc7-d7aa-43f6-a088-5069e9e90305-35305123",
        "queryResult": {
          "queryText": "hi",
          "action": "input.welcome",
          "parameters": {},
          "allRequiredParamsPresent": true,
          "fulfillmentText": "Hi, this is Dummy Express, your specialist in international shipping.",
          "fulfillmentMessages": [
            {
              "text": {
                "text": [
                  "Hi, this is Dummy Express, your specialist in international shipping!"
                ]
              },
              "platform": "FACEBOOK"
            },
            {
              "text": {
                "text": [
                  "Hi, this is Dummy Express, your specialist in international shipping. I can track a package if you provide a 10 digit shipment number. I can also provide rate quotes."
                ]
              },
              "platform": "LINE"
            },
            {
              "quickReplies": {
                "quickReplies": [
                  "Track a package",
                  "Manage delivery",
                  "Pay duties",
                  "Commercial invoice",
                  "Get a quote"
                ]
              },
              "platform": "FACEBOOK"
            },
            {
              "platform": "ACTIONS_ON_GOOGLE",
              "simpleResponses": {
                "simpleResponses": [
                  {
                    "ssml": "<speak><prosody rate=\"115%\"><s>Welcome to Dummy Express, your specialist in international shipping.</s>\n<s>I can track a package or provide rate quotes.</s></prosody></speak>"
                  }
                ]
              }
            },
            {
              "quickReplies": {
                "quickReplies": [
                  "Track a package",
                  "Manage delivery",
                  "Pay duties",
                  "Commercial invoice",
                  "Get a quote"
                ]
              },
              "platform": "SKYPE"
            },
            {
              "text": {
                "text": [
                  "Hi, this is Dummy Express, your specialist in international shipping. I can track a package if you provide a 10 digit shipment number. I can also provide rate quotes."
                ]
              }
            }
          ],
          "outputContexts": [
            {
              "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
              "lifespanCount": 1
            }
          ],
          "intent": {
            "name": "projects/express-cs-dummy/agent/intents/b1967059-d268-4c12-861d-9d71e710b123",
            "displayName": "Generic|BIT|0|Welcome|Gen"
          },
          "intentDetectionConfidence": 1,
          "languageCode": "en",
          "sentimentAnalysisResult": {
            "queryTextSentiment": {
              "score": 0.3,
              "magnitude": 0.3
            }
          }
        }
      }
      "#;       

    #[test]
    fn test_process_assertion_response_check_str_equals() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            String::from("queryResult.action"), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::StrVal(String::from("input.welcome"))
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            String::from("queryResult.action"), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::StrVal(String::from("foo.bar"))
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::StrVal("foo.bar".to_string())
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected value 'foo.bar' does not match real value: 'input.welcome' for expression: queryResult.action");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve string value for expression: queryResult.action.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }

    #[test]
    fn test_process_assertion_response_check_str_includes() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action".to_string(), 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::StrVal("nput.welcom".to_string())
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action".to_string(), 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::StrVal("foo.bar".to_string())
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::StrVal("foo.bar".to_string())
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected value 'foo.bar' not included in real value: 'input.welcome' for expression: queryResult.action");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve string value for expression: queryResult.action.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }    

    #[test]
    fn test_process_assertion_response_check_str_not_equals() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal("foo.bar".to_string())
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal("input.welcome".to_string())
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal("input.welcome".to_string())
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected value 'input.welcome' does match real value: 'input.welcome' for expression: queryResult.action");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve string value for expression: queryResult.action.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }    

    #[test]
    fn test_process_assertion_response_check_str_length() {
        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action".to_string(), 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::StrVal("input.welcome".to_string())
        );

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Operator length not allowed for string value of expression: 'queryResult.action'. If value is '4' use 4 instead.");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }
    }        

    #[test]
    fn test_process_assertion_response_check_str_json_equals_arrays() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts".to_string(), 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
            [
                {
                  "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 1
                }
            ]            
            "#.to_string())
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts".to_string(), 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
            [
                {
                  "name2": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 2
                }
            ]            
            "#.to_string())
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal(r#"[{"foo": "bar"}]"#.to_string())
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        println!("{}", err.message);
                        /*    
                        should get something like:

                        Arrays not matching for expression 'queryResult.outputContexts'. Error: json atoms at path "[0].lifespanCount" are not equal:
                        lhs:
                            1
                        rhs:
                            2
                        json atom at path "[0].name2" is missing from lhs
                        */

                        assert_eq!(err.message.contains("Arrays not matching for expression 'queryResult.outputContexts'"), true);
                        assert_eq!(err.message.contains(r#"json atoms at path "[0].lifespanCount" are not equal"#), true);
                        assert_eq!(err.message.contains(r#"json atom at path "[0].name2" is missing from lhs"#), true);
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve string value for expression: queryResult.outputContexts.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }    

    #[test]
    fn test_process_assertion_response_check_str_json_equals_objects() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0]".to_string(), 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
                {
                  "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 1
                }
            "#.to_string())
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0]".to_string(), 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
                {
                  "name2": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 2
                }
            "#.to_string())
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal(r#"{"foo": "bar"}"#.to_string())
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        println!("{}", err.message);
                        /*    
                        should get something like:

                        Objects not matching for expression 'queryResult.outputContexts[0]'. Error: json atoms at path ".lifespanCount" are not equal:
                            lhs:
                                1
                            rhs:
                                2

                        json atom at path ".name" is missing from rhs

                        json atom at path ".name2" is missing from lhs
                        */

                        assert_eq!(err.message.contains("Objects not matching for expression 'queryResult.outputContexts[0]'"), true);
                        assert_eq!(err.message.contains(r#"json atoms at path ".lifespanCount" are not equal:"#), true);
                        assert_eq!(err.message.contains(r#"json atom at path ".name" is missing from rhs"#), true);
                        assert_eq!(err.message.contains(r#"json atom at path ".name2" is missing from lhs"#), true);
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve string value for expression: queryResult.outputContexts.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }    


    #[test]
    fn test_process_assertion_response_check_bool_equals() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected value (false) does not match real value: (true) for expression: queryResult.allRequiredParamsPresent");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve boolean value (true) for expression: queryResult.allRequiredParamsPresent.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }    

    #[test]
    fn test_process_assertion_response_check_bool_not_equals() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected value (false), got instead value: (true) for expression: queryResult.allRequiredParamsPresent");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve boolean value (false) for expression: queryResult.allRequiredParamsPresent.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }    

    #[test]
    fn test_process_assertion_response_check_bool_includes() {
        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Operator includes not allowed for boolean value of expression: queryResult.allRequiredParamsPresent");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }
    }    

    #[test]
    fn test_process_assertion_response_check_bool_json_equals() {
        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Operator jsonequals not allowed for boolean value of expression: queryResult.allRequiredParamsPresent");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }
    }        
    
    #[test]
    fn test_process_assertion_response_check_bool_length() {
        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Operator length not allowed for boolean value of expression: queryResult.allRequiredParamsPresent");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }
    }     
    
    #[test]
    fn test_process_assertion_response_check_num_equals() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount".to_string(), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount".to_string(), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::NumVal(2_f64)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected value (2) does not match real value: (1) for expression: queryResult.outputContexts[0].lifespanCount");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve numerical value for expression: queryResult.outputContexts[0].lifespanCount.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }  
    
    #[test]
    fn test_process_assertion_response_check_num_not_equals() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::NumVal(2.0)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::NumVal(1.0)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );
        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected value not equal to (1) got value: (1) for expression: queryResult.outputContexts[0].lifespanCount");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve numerical value for expression: queryResult.outputContexts[0].lifespanCount.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        
    }  
    
    #[test]
    fn test_process_assertion_response_check_num_includes() {
        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::NumVal(1.0)
        );

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Operator includes not allowed for numeric value of expression: queryResult.allRequiredParamsPresent");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }
    }    
    
    #[test]
    fn test_process_assertion_response_check_num_json_equals() {
        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent".to_string(), 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::NumVal(1.0)
        );

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Operator jsonequals not allowed for numeric value of expression: queryResult.allRequiredParamsPresent");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }
    }    
    
    #[test]
    fn test_process_assertion_response_check_num_length() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts".to_string(), 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts".to_string(), 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(2_f64)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts.does.not.exists".to_string(), 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        let check_ko_3: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0]".to_string(), 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        
        assert_eq!(TestSuiteExecutor::process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match TestSuiteExecutor::process_assertion_response_check(&check_ko_1, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Expected array length 2, got 1 for expression: queryResult.outputContexts");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_2, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Unable to retrieve array value for expression: queryResult.outputContexts.does.not.exists");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }        

        match TestSuiteExecutor::process_assertion_response_check(&check_ko_3, JSON) {
            Err(err) => {
                match *err.kind {
                    ErrorKind::InvalidTestAssertionResponseCheckEvaluation => {
                        assert_eq!(err.message, "Operator length allowed for array expressions only. Expression: queryResult.outputContexts[0]");
                    },
                    _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error, got different error")
                }
            },
            _ => assert!(false, "Expected InvalidTestAssertionResponseCheckEvaluation error")
        }                
    }    

}