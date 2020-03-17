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

#[derive(Debug)]
pub struct JsonParsingError(String);

impl fmt::Display for JsonParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "JsonParsingError occurred: {}", &self.0)
    }
}

impl From<JmespathError> for JsonParsingError {
    fn from(error: JmespathError) -> Self {
      JsonParsingError(format!("error when parsing json: {}", error))
    }    
}

impl From<String> for JsonParsingError {
  fn from(error: String) -> Self {
    JsonParsingError(format!("error when parsing json: {}", error))
  }    
}


impl Error for JsonParsingError {}

pub fn json_extract_string_value (json: &str, value: &str) -> Result<Variable, JsonParsingError> {
    let expr = jmespath::compile(value)?;
    let data = jmespath::Variable::from_json(json)?;
    let result = expr.search(data)?;

    match result.as_string() {
      Some(str_value) => return Ok(Variable::String(str_value.to_owned())),
      None => return Ok(Variable::Null)
    }
}

pub fn json_extract_number_value (json: &str, value: &str) -> Result<Variable, JsonParsingError> {
  let expr = jmespath::compile(value)?;
  let data = jmespath::Variable::from_json(json)?;
  let result = expr.search(data)?;

  match result.as_number() {
    Some(number_value) => return Ok(Variable::Number(number_value)),
    None => return Ok(Variable::Null)
  }
}


pub fn json_extract_object_value (json: &str, parent: &str, key: &str) -> Result<Rc<Variable>, JsonParsingError> {
  let expr = jmespath::compile(parent)?;
  let data = jmespath::Variable::from_json(json)?;
  let result = expr.search(data)?;

  match result.as_object() {
    Some(map_value) => {
      if map_value.contains_key(key) {
        let rc_var_ref_opt: Option<&Rc<Variable>> = map_value.get(key);
        match rc_var_ref_opt {
          None => return Err(JsonParsingError(format!("json_extract_object_value failed. Key {} not found in {}", key, parent))),
          Some(rc_var_ref) => return Ok(rc_var_ref.clone()) // cannot move out Variable from Rc -> need to clone
        }
      }
      return Err(JsonParsingError(format!("json_extract_object_value failed. Yaml doc does not contain {}", key)));
    },
    None => return Err(JsonParsingError(format!("json_extract_object_value failed. Yaml doc does not contain {}", parent)))
  }
}

pub fn json_unwrap_string_value_or_panic (value: Variable) -> String {
  if let Variable::String(str_val) = value {
    str_val
  } else {
    panic!(format!("json_unwrap_string_value_or_panic failed for value {}", value ));
  }
}

pub fn json_unwrap_number_value_or_panic (value: Variable) -> f64 {
  if let Variable::Number(number_value) = value {
    number_value
  } else {
    panic!(format!("json_unwrap_number_value_or_panic failed for value {}", value ));
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

    // simple string parameter extraction  
    #[test]
    fn test_string_extraction() {
        let result = json_extract_string_value(JSON, "queryResult.action").unwrap();
        let value = json_unwrap_string_value_or_panic(result);
        assert_eq!("input.welcome", value);
    }

    // string parameter extraction + partial match + accessing JSON arrays
    #[test]
    fn test_arrays_substrings() {
        let result = json_extract_string_value(JSON, "queryResult.fulfillmentText").unwrap();
        let value = json_unwrap_string_value_or_panic(result);
        assert_eq!("Hi, this is Dummy Express, your specialist in international shipping.", value);

        let result = json_extract_string_value(JSON, "queryResult.fulfillmentMessages[0].text.text[0]").unwrap();
        let value = json_unwrap_string_value_or_panic(result);
        assert_eq!("Hi, this is Dummy Express, your specialist in international shipping!", value);

        let result = json_extract_string_value(JSON, "queryResult.fulfillmentMessages[2].quickReplies.quickReplies[1]").unwrap();
        let value = json_unwrap_string_value_or_panic(result);
        assert_eq!("Manage delivery", value);        
        assert!(value.contains("nage deli"));        
    }    

    #[test]
    fn test_contexts() {
      let result = json_extract_string_value(JSON, "queryResult.outputContexts[0].name").unwrap();
      let value = json_unwrap_string_value_or_panic(result);
      assert_eq!("projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt", value);

      let result = json_extract_number_value(JSON, "queryResult.outputContexts[0].lifespanCount").unwrap();
      let value = json_unwrap_number_value_or_panic(result);
      assert_eq!(1, value as u32);
    }


    #[test]
    fn test_json_compare1() {
      let v = json!({ "an": "object" });
      let v2 = json!({ "an": "object" });
      assert_json_eq!(v, v2);

      let v = json!(r#"{ "an": "object" }"#);
      let v2 = json!(r#"{ "an": "object" }"#);
      assert_json_eq!(v, v2);
    }    

    #[test]
    fn test_json_compare2() {
      // https://gist.github.com/nwtnni/a769fa093c4118c9716957957dcee332
      let result = json_extract_object_value(JSON, "queryResult", "intent").unwrap();
      let value_real = json!(result);
      let value_expected = json!({
        "name": "projects/express-cs-dummy/agent/intents/b1967059-d268-4c12-861d-9d71e710b123",
        "displayName": "Generic|BIT|0|Welcome|Gen"
      });

      assert_json_eq!(value_real, value_expected);
    }

    #[test]
    fn test_json_compare3() {
      let result = json_extract_object_value(JSON, "queryResult", "intent").unwrap();
      let value_real = json!(result);
      let value_expected_str = r#"{
        "name": "projects/express-cs-dummy/agent/intents/b1967059-d268-4c12-861d-9d71e710b123",
        "displayName": "Generic|BIT|0|Welcome|Gen"
      }"#;

      assert_json_eq!(value_real, from_str(value_expected_str).unwrap());
    }

    #[test]
    #[ignore]
    fn test_json_compare4() {
      // failing now, we need to find a way how to extract whole array
      let result = json_extract_object_value(JSON, "queryResult.outputContexts[0]", "").unwrap();
      let value_real = json!(result);
      let value_expected_str = r#"{
        "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
        "lifespanCount": 1
      }"#;

      assert_json_eq!(value_real, from_str(value_expected_str).unwrap());
    }    
}
