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
pub fn process_assertion_respopnse_check(response_check: &TestAssertionResponseCheck, response: &str) -> Result<()> {
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
                            let error_message = format!("Expected value ({}) does not match real value: ({}) for expression {}", bool_val_expected, bool_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve boolean value {} for expression: '{}'", bool_val_expected, response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

                TestAssertionResponseCheckOperator::Includes => {
                    let error_message = format!("Operator includes not allowed for boolean value of expression: '{}'", response_check.expression);
                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                },

                TestAssertionResponseCheckOperator::JsonEquals => {
                    let error_message = format!("Operator jsonequals not allowed for boolean value of expression: '{}'", response_check.expression);
                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                },

                TestAssertionResponseCheckOperator::Length => {
                    let error_message = format!("Operator length not allowed for boolean value of expression: '{}'", response_check.expression);
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
                            let error_message = format!("Expected value ({}) does match real value: ({}) for expression {}", bool_val_expected, bool_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve boolean value {} for expression: '{}'", !bool_val_expected, response_check.expression);
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
                            let error_message = format!("Expected value '{}' does not match real value: '{}' for expression {}", str_val_expected, str_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve string value for expression: '{}'", response_check.expression);
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
                            let error_message = format!("Expected value '{}' not included in real value: '{}' for expression {}", str_val_expected, str_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve string value for expression: '{}'", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

                TestAssertionResponseCheckOperator::JsonEquals => {
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;

                    // TBD... interpret real value as object or array and compare with string value of response check
                    if JsonParser::get_jmespath_var_type(&search_result) == Some(JmespathType::Array) {
                       
                        let value = JsonParser::extract_as_array(&search_result);
                        if let Some(array_val_real) = value {
                            let json_comparison_result = JsonParser::compare_array_with_str(array_val_real, str_val_expected);

                            match json_comparison_result {
                                Ok(str_val) if str_val == "__OK__" => {},
                                Ok(err_msg) => {
                                    let error_message = format!("Arrays not matching for expression '{}' Error: {}", response_check.expression, err_msg);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                },
                                Err(error) => {
                                    let error_message = format!("Arrays not matching for expression '{}' Error: {}", response_check.expression, error);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                }
                            }
                        } else {
                            let error_message = format!("Unable to retrieve string value for expression: '{}'", response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    
                    } else if JsonParser::get_jmespath_var_type(&search_result) == Some(JmespathType::Object) {
                        let value = JsonParser::extract_as_object(&search_result);

                        if let Some(obj_val_real) = value {
                            let json_comparison_result = JsonParser::compare_object_with_str(obj_val_real, str_val_expected);

                            match json_comparison_result {
                                Ok(str_val) if str_val == "__OK__" => {},
                                Ok(err_msg) => {
                                    let error_message = format!("Objects not matching for expression '{}' Error: {}", response_check.expression, err_msg);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                },
                                Err(error) => {
                                    let error_message = format!("Objects not matching for expression '{}' Error: {}", response_check.expression, error);
                                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                                }
                            }
                        } else {
                            let error_message = format!("Unable to retrieve string value for expression: '{}'", response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                        
                    } else {
                        let error_message = format!("Cannot apply jsonequals oeprator. Retrieved value is neither object nor array for expression: '{}'", response_check.expression);
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
                        if str_val_real == str_val_expected {
                            return Ok(());
                        } else {
                            let error_message = format!("Expected value ({}) does match real value: ({}) for expression {}", str_val_expected, str_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve string value for expression: '{}'", response_check.expression);
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
                            let error_message = format!("Expected value ({}) does not match real value: ({}) for expression {}", num_val_expected, num_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve numerical value for expression: '{}'", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                    }
                },

                TestAssertionResponseCheckOperator::Includes => {
                    let error_message = format!("Operator includes not allowed for numeric value of expression: '{}'", response_check.expression);
                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                },

                TestAssertionResponseCheckOperator::JsonEquals => {
                    let error_message = format!("Operator jsonequals not allowed for numeric value of expression: '{}'", response_check.expression);
                    return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                },

                TestAssertionResponseCheckOperator::Length => {
                    // TBD: we will support length of arrays only, not lenght of strings or number of digits in number
                    let parser = JsonParser::new(response);
                    let search_result = parser.search(response_check.expression)?;

                    if JsonParser::get_jmespath_var_type(&search_result) == Some(JmespathType::Array) {
                        let value = JsonParser::extract_as_array(&search_result);
                        if let Some(arr_value) = value {
                            if arr_value.len() == num_val_expected as usize { // TODO: num value in response check should be usize, f64 does not make sense if used only for array length comparison
                                return Ok(());
                            } else {
                                let error_message = format!("Expected array length {}, got {} for expression: '{}'", num_val_expected, arr_value.len(), response_check.expression);
                                return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                            }
                        } else {
                            let error_message = format!("Unable to retrieve array value for expression: '{}'", response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Operator length allowed for array exoressions only. Expression: '{}'", response_check.expression);
                        return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
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
                            let error_message = format!("Expected value ({}) does equal real value: ({}) for expression {}", num_val_expected, num_val_real, response_check.expression);
                            return Err(new_error(ErrorKind::InvalidTestAssertionResponseCheckEvaluation, error_message, None));
                        }
                    } else {
                        let error_message = format!("Unable to retrieve numerical value for expression: '{}'", response_check.expression);
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
    