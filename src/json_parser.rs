use jmespath;
use std::fmt;
use std::error::Error;
use jmespath::JmespathError;
use jmespath::Variable;
use serde_json::json;
use serde_json::from_str;
use std::collections::HashMap;
use assert_json_diff::assert_json_eq;
use std::rc::Rc;
use crate::errors::{Result, ErrorKind};
type StdResult<T, E> = std::result::Result<T, E>;

// JMESPath types.
// replacement for jmespath::variable::JmespathType
// which is private (probably bug of library implementation)
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum JmespathType {
    Null,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Expref,
}

impl fmt::Display for JmespathType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(fmt,
               "{}",
               match *self {
                   JmespathType::Null => "null",
                   JmespathType::String => "string",
                   JmespathType::Number => "number",
                   JmespathType::Boolean => "boolean",
                   JmespathType::Array => "array",
                   JmespathType::Object => "object",
                   JmespathType::Expref => "expref",
               })
    }
}

pub struct JsonParser<'a> {
  json: &'a str
}

impl<'a> JsonParser<'a> {
  pub fn new(json: &'a str) -> Self {
    JsonParser {
      json
    }
  }

  pub fn search(&self, expression: &str) -> Result<Rc<Variable>> {
    let jmespath_expr = jmespath::compile(expression)?;
    let data = jmespath::Variable::from_json(&self.json)?;
    let rc_var = jmespath_expr.search(data)?;
    Ok(rc_var)
  }

  pub fn extract_as_string(variable: &'a Rc<Variable>) -> Option<&'a str> {
    match variable.as_string() {
      Some(str_value) => Some(str_value),
      _ => None
    }
  }

  pub fn extract_as_number(variable: &'a Rc<Variable>) -> Option<f64> {
    match variable.as_number() {
      Some(number_value) => Some(number_value),
      _ => None
    }
  }  

  pub fn extract_as_bool(variable: &'a Rc<Variable>) -> Option<bool> {
    match variable.as_boolean() {
      Some(bool_value) => Some(bool_value),
      _ => None
    }
  }    

  pub fn extract_as_array(variable: &'a Rc<Variable>) -> Option<Vec<Rc<Variable>>> {
    match variable.as_array() {
      Some(array_value) => Some(array_value.to_vec()),
      _ => None
    }
  }      

  pub fn extract_as_object(variable: &'a Rc<Variable>) -> Option<Rc<Variable>> {
    if (variable.is_object() == true) {
      Some(variable.clone())
    } else {
      None
    }
  }  
  
  pub fn get_jmespath_var_type(variable: &'a Rc<Variable>) -> Option<JmespathType> {
    if variable.is_null() {
      return Some(JmespathType::Null)
    }
  
    if variable.is_string() {
      return Some(JmespathType::String)
    }
  
    if variable.is_number() {
      return Some(JmespathType::Number)
    }
    if variable.is_boolean() {
      return Some(JmespathType::Boolean)
    }
  
    if variable.is_array() {
      return Some(JmespathType::Array)
    }
  
    if variable.is_object() {
      return Some(JmespathType::Object)
    }
  
    if variable.is_expref() {
      return Some(JmespathType::Expref)
    }
    
    None
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
      fn test_json_compare() {
        let v = json!({ "an": "object" });
        let v2 = json!({ "an": "object" });
        assert_json_eq!(v, v2);
  
        let v = json!(r#"{ "an": "object" }"#);
        let v2 = json!(r#"{ "an": "object" }"#);
        assert_json_eq!(v, v2);
      }    
  

    // simple string parameter extraction  
    #[test]
    fn test_json_extract_string_value_1() {
        let parser = JsonParser::new(JSON);
        let search_result = parser.search("queryResult.action").unwrap();
        let value = JsonParser::extract_as_string(&search_result);
        assert_eq!(value, Some("input.welcome"));
    }

    #[test]
    fn test_json_extract_string_value_2() {
        let parser = JsonParser::new(JSON);

        let mut search_result = parser.search("queryResult.fulfillmentText").unwrap();
        let mut value = JsonParser::extract_as_string(&search_result);
        assert_eq!(value, Some("Hi, this is Dummy Express, your specialist in international shipping."));


        search_result = parser.search("queryResult.fulfillmentMessages[0].text.text[0]").unwrap();
        value = JsonParser::extract_as_string(&search_result);
        assert_eq!(value, Some("Hi, this is Dummy Express, your specialist in international shipping!"));

        search_result = parser.search("queryResult.fulfillmentMessages[2].quickReplies.quickReplies[1]").unwrap();
        value = JsonParser::extract_as_string(&search_result);
        assert_eq!(value, Some("Manage delivery"));

        match value {
          Some(val) => assert!(val.contains("nage deli")),
          _ => assert!(false, r#"value should contain "nage deli""#)
        }
    }    

    #[test]
    fn test_json_extract_string_value_3() {
      let parser = JsonParser::new(JSON);

      let search_result = parser.search("queryResult.outputContexts[0].name").unwrap();
      let value = JsonParser::extract_as_string(&search_result);
      assert_eq!(value, Some("projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt"));
    }

    #[test]
    fn test_json_extract_number_value() {
      let parser = JsonParser::new(JSON);

      let search_result = parser.search("queryResult.outputContexts[0].lifespanCount").unwrap();
      let value = JsonParser::extract_as_number(&search_result);
      assert_eq!(value, Some(1.0));
    }

    #[test]
    fn test_json_extract_boolean_value() {
      let parser = JsonParser::new(JSON);

      let search_result = parser.search("queryResult.allRequiredParamsPresent").unwrap();
      let value = JsonParser::extract_as_bool(&search_result);
      assert_eq!(value, Some(true));
    }    

    #[test]
    fn test_json_extract_object_value_1() {

      let parser = JsonParser::new(JSON);

      let search_result = parser.search("queryResult.intent").unwrap();
      let value_real = JsonParser::extract_as_object(&search_result);

      let value_expected = json!({
        "name": "projects/express-cs-dummy/agent/intents/b1967059-d268-4c12-861d-9d71e710b123",
        "displayName": "Generic|BIT|0|Welcome|Gen"
      });

      if let Some(_value_real) = value_real {
        assert_json_eq!(json!(_value_real), value_expected);
      } else {
        assert!(false, "unexpected value returned")
      }
    }

    #[test]
    fn test_json_extract_object_value_2() {

      let parser = JsonParser::new(JSON);

      let search_result = parser.search("queryResult.intent").unwrap();
      let value_real = JsonParser::extract_as_object(&search_result);

      // we can provided expected value as string as well
      let value_expected = r#"{
        "name": "projects/express-cs-dummy/agent/intents/b1967059-d268-4c12-861d-9d71e710b123",
        "displayName": "Generic|BIT|0|Welcome|Gen"
      }"#;

      if let Some(_value_real) = value_real {
        assert_json_eq!(json!(_value_real), from_str(value_expected).unwrap());
      } else {
        assert!(false, "unexpected value returned")
      }
    }

    #[test]
    #[ignore]
    fn test_json_extract_object_value_and_filter() {
      
      let parser = JsonParser::new(JSON);

      // filtering seems not to work in jmespath :(
      // https://jmespath.org/examples.html#filters-and-multiselect-lists
      // https://jmespath.org/proposals/improved-filters.html
      let search_result = parser.search("queryResult.fulfillmentMessages[?platform == 'FACEBOOK']").unwrap();
      let value_real = JsonParser::extract_as_object(&search_result);

      // we can provided expectd value as string as well
      let value_expected = r#"[
        {
          "text": {
            "text": [
              "Hi, this is Dummy Express, your specialist in international shipping!"
            ]
          },
          "platform": "FACEBOOK"
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
        }
      ]"#;

      if let Some(_value_real) = value_real {
        assert_json_eq!(json!(_value_real), from_str(value_expected).unwrap());
      } else {
        assert!(false, "unexpected value returned")
      }
    }    


    #[test]
    fn test_json_extract_array_value() {

      let parser = JsonParser::new(JSON);

      let search_result = parser.search("queryResult.outputContexts").unwrap();
      let value_real = JsonParser::extract_as_array(&search_result);

      let value_expected = r#"[{
        "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
        "lifespanCount": 1
      }]"#;


      if let Some(_value_real) = value_real {
        assert_json_eq!(json!(_value_real), from_str(value_expected).unwrap());
        assert_eq!(_value_real.len(), 1);
      } else {
        assert!(false, "unexpected value returned")
      }
    }    

    #[test]
    fn test_get_jmespath_var_type() {

      let mut parser = JsonParser::new(JSON);

      let mut search_result = parser.search("queryResult.allRequiredParamsPresentDOESNOTEXIST").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::Null));

      search_result = parser.search("queryResult.outputContexts[0].name").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::String));

      search_result = parser.search("queryResult.outputContexts[0].lifespanCount").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::Number));

      search_result = parser.search("queryResult.allRequiredParamsPresent").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::Boolean));

      search_result = parser.search("queryResult.outputContexts").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::Array));

      search_result = parser.search("queryResult.outputContexts[0]").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::Object));

      search_result = parser.search("queryResult").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::Object));

      search_result = parser.search("queryResult").unwrap();
      assert_eq!(JsonParser::get_jmespath_var_type(&search_result), Some(JmespathType::Object));
      
      parser = JsonParser::new("");
      let search_result = parser.search("queryResult.outputContexts[0]");

      match search_result {
        Ok(_) => assert!(false, "unexpected value returned by get_jmespath_var_type, expected error!"),
        Err(err) => {
          match *err.kind {
            ErrorKind::GenericError(err_msg) => {
              println!("hey");
              assert!(err_msg.contains("GenericError: EOF while parsing a value at line 1 column 0"), "wrong error message retrieved");
            },
            _ => assert!(false, "Expected generic error")
          }
        }
      }      
    }
}


