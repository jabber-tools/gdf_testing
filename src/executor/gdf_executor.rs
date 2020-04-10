use reqwest;
use guid_create::GUID;
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

use crate::executor::{TestExecutor, AssertionExecutionContext};

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

pub fn process_assertion(context: &AssertionExecutionContext) -> Result<String> {
    print!("saying '{}'", context.assertion.user_says);
    let resp = match context.suite_type {
        TestSuiteType::DHLVAP => invoke_vap(context),
        TestSuiteType::DialogFlow => invoke_gdf(context),
    }?;
    println!(" - ok!");
    Ok(resp)
}

// highly unoptimized and naive version of method for checking assertion response checks
// we will have to do something with this terrifying code...
pub fn process_assertion_response_check(response_check: &TestAssertionResponseCheck, response: &str) -> Result<()> {
    match response_check.value {
        
        TestAssertionResponseCheckValue::BoolVal(bool_val_expected) => {
            
            match response_check.operator {

                TestAssertionResponseCheckOperator::Equals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;

                    let value = JsonParser::extract_as_bool(&search_result);
                    if let Some(bool_val_real) = value {
                        if bool_val_real == bool_val_expected {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value ({}) does not match real value: ({}) for expression: {}", bool_val_expected, bool_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve boolean value ({}) for expression: {}", bool_val_expected, response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

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

                TestAssertionResponseCheckOperator::NotEquals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;
                    let value = JsonParser::extract_as_bool(&search_result);
                    if let Some(bool_val_real) = value {
                        if bool_val_real != bool_val_expected {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value ({}), got instead value: ({}) for expression: {}", !bool_val_expected, bool_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve boolean value ({}) for expression: {}", !bool_val_expected, response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                }

            }

        },

        TestAssertionResponseCheckValue::StrVal(str_val_expected) => {
            
            match response_check.operator {

                TestAssertionResponseCheckOperator::Equals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;
                    let value = JsonParser::extract_as_string(&search_result);
                    if let Some(str_val_real) = value {
                        if str_val_real == str_val_expected {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value '{}' does not match real value: '{}' for expression: {}", str_val_expected, str_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

                TestAssertionResponseCheckOperator::Includes => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;
                    let value = JsonParser::extract_as_string(&search_result);
                    if let Some(str_val_real) = value {
                        if str_val_real.contains(str_val_expected) == true {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value '{}' not included in real value: '{}' for expression: {}", str_val_expected, str_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

                TestAssertionResponseCheckOperator::JsonEquals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;

                    if JsonParser::get_jmespath_var_type(&search_result) == Some(JmespathType::Array) {
                       
                        let value = JsonParser::extract_as_array(&search_result);
                        if let Some(array_val_real) = value {
                            let json_comparison_result = JsonParser::compare_array_with_str(&array_val_real, str_val_expected);

                            match json_comparison_result {
                                Ok(str_val) if str_val == "__OK__" => {},
                                Ok(err_msg) => {
                                    let error_message = format!("Arrays not matching for expression '{}'. Error: {}", response_check.expression, err_msg);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                },
                                Err(error) => {
                                    let error_message = format!("Arrays not matching for expression '{}'. Error: {}", response_check.expression, error);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                }
                            }
                        } else {
                            let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    
                    } else if JsonParser::get_jmespath_var_type(&search_result) == Some(JmespathType::Object) {
                        let value = JsonParser::extract_as_object(&search_result);

                        if let Some(obj_val_real) = value {
                            let json_comparison_result = JsonParser::compare_object_with_str(&obj_val_real, str_val_expected);

                            match json_comparison_result {
                                Ok(str_val) if str_val == "__OK__" => {},
                                Ok(err_msg) => {
                                    let error_message = format!("Objects not matching for expression '{}'. Error: {}", response_check.expression, err_msg);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                },
                                Err(error) => {
                                    let error_message = format!("Objects not matching for expression '{}'. Error: {}", response_check.expression, error);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                }
                            }
                        } else {
                            let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                        
                    } else {
                        let error_message = format!("Cannot apply jsonequals operator. Retrieved value is neither object nor array for expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

                TestAssertionResponseCheckOperator::Length => {
                    let error_message = format!("Operator length not allowed for string value of expression: '{}'. If value is '4' use 4 instead.", response_check.expression);
                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                },

                TestAssertionResponseCheckOperator::NotEquals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;
                    let value = JsonParser::extract_as_string(&search_result);
                    if let Some(str_val_real) = value {
                        if str_val_real != str_val_expected {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value '{}' does match real value: '{}' for expression: {}", str_val_expected, str_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve string value for expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                }

            }

        },

        TestAssertionResponseCheckValue::NumVal(num_val_expected) => {
            
            match response_check.operator {
                
                TestAssertionResponseCheckOperator::Equals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;
                    let value = JsonParser::extract_as_number(&search_result);
                    if let Some(num_val_real) = value {
                        if num_val_real == num_val_expected {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value ({}) does not match real value: ({}) for expression: {}", num_val_expected, num_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve numerical value for expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

                TestAssertionResponseCheckOperator::Includes => {
                    let error_message = format!("Operator includes not allowed for numeric value of expression: {}", response_check.expression);
                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                },

                TestAssertionResponseCheckOperator::JsonEquals => {
                    let error_message = format!("Operator jsonequals not allowed for numeric value of expression: {}", response_check.expression);
                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                },

                TestAssertionResponseCheckOperator::Length => {
                    // we do support length of arrays only, not lenght of strings or number of digits in number!
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;

                    match JsonParser::get_jmespath_var_type(&search_result) {
                        Some(JmespathType::Array) => /* array type */ {
                            let value = JsonParser::extract_as_array(&search_result);
                            if let Some(arr_value) = value {
                                if arr_value.len() == num_val_expected as usize { // TODO: num value in response check should be usize, f64 does not make sense if used only for array length comparison
                                    return Ok(());
                                } else {
                                    let error_message = format!("Expected array length {}, got {} for expression: {}", num_val_expected, arr_value.len(), response_check.expression);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                }
                            } else {
                                let error_message = format!("Unable to retrieve array value for expression: {}", response_check.expression);
                                return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                            }
    
                        },
                        /* no type, i.e. expression does not match any value in json */
                        Some(JmespathType::Null) |
                        None =>  {
                            let error_message = format!("Unable to retrieve array value for expression: {}", response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        },
                        Some(_) => /* some other type, e.g. object*/ {
                            let error_message = format!("Operator length allowed for array expressions only. Expression: {}", response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    }
                },

                TestAssertionResponseCheckOperator::NotEquals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;
                    let value = JsonParser::extract_as_number(&search_result);
                    if let Some(num_val_real) = value {
                        if num_val_real != num_val_expected {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value not equal to ({}) got value: ({}) for expression: {}", num_val_expected, num_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve numerical value for expression: {}", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                }
            }

        }
    }

    Ok(())
}

pub fn invoke_gdf(context: &AssertionExecutionContext) -> Result<String> {
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
    Ok(resp)
}

pub fn invoke_vap(context: &AssertionExecutionContext) -> Result<String> {
    // TBD...
    println!("invoking VAP {}", context.assertion.user_says);
    Ok("tbd...".to_owned())
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

    // cargo test -- --show-output test_process_test
    #[test]
    #[ignore]
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
                  botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
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

    #[test]
    fn test_process_assertion_response_check_str_equals() {
        let check_ok: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::StrVal("input.welcome")
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::StrVal("foo.bar")
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action.does.not.exists", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::StrVal("foo.bar")
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.action", 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::StrVal("nput.welcom")
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action", 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::StrVal("foo.bar")
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action.does.not.exists", 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::StrVal("foo.bar")
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.action", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal("foo.bar")
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal("input.welcome")
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.action.does.not.exists", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal("input.welcome")
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.action", 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::StrVal("input.welcome")
        );

        match process_assertion_response_check(&check_ko_1, JSON) {
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
            "queryResult.outputContexts", 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
            [
                {
                  "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 1
                }
            ]            
            "#)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts", 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
            [
                {
                  "name2": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 2
                }
            ]            
            "#)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts.does.not.exists", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal(r#"[{"foo": "bar"}]"#)
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.outputContexts[0]", 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
                {
                  "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 1
                }
            "#)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0]", 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::StrVal(r#"
                {
                  "name2": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                  "lifespanCount": 2
                }
            "#)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts.does.not.exists", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::StrVal(r#"{"foo": "bar"}"#)
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent.does.not.exists", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.allRequiredParamsPresent.does.not.exists", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::BoolVal(true)
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        match process_assertion_response_check(&check_ko_1, JSON) {
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
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        match process_assertion_response_check(&check_ko_1, JSON) {
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
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::BoolVal(false)
        );

        match process_assertion_response_check(&check_ko_1, JSON) {
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
            "queryResult.outputContexts[0].lifespanCount", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::NumVal(2_f64)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount.does.not.exists", 
            TestAssertionResponseCheckOperator::Equals,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.outputContexts[0].lifespanCount", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::NumVal(2.0)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::NumVal(1.0)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0].lifespanCount.does.not.exists", 
            TestAssertionResponseCheckOperator::NotEquals,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );
        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::Includes,
            TestAssertionResponseCheckValue::NumVal(1.0)
        );

        match process_assertion_response_check(&check_ko_1, JSON) {
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
            "queryResult.allRequiredParamsPresent", 
            TestAssertionResponseCheckOperator::JsonEquals,
            TestAssertionResponseCheckValue::NumVal(1.0)
        );

        match process_assertion_response_check(&check_ko_1, JSON) {
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
            "queryResult.outputContexts", 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        let check_ko_1: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts", 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(2_f64)
        );

        let check_ko_2: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts.does.not.exists", 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        let check_ko_3: TestAssertionResponseCheck = TestAssertionResponseCheck::new(
            "queryResult.outputContexts[0]", 
            TestAssertionResponseCheckOperator::Length,
            TestAssertionResponseCheckValue::NumVal(1_f64)
        );

        
        assert_eq!(process_assertion_response_check(&check_ok, JSON).unwrap(), ());
        
        match process_assertion_response_check(&check_ko_1, JSON) {
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

        match process_assertion_response_check(&check_ko_2, JSON) {
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

        match process_assertion_response_check(&check_ko_3, JSON) {
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
    