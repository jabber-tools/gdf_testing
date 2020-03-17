use jmespath;
use std::fmt;
use std::error::Error;
use jmespath::JmespathError;
use jmespath::Variable;

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

pub fn json_extract_value (json: &str, value: &str) -> Result<Variable, JsonParsingError> {
    let expr = jmespath::compile("queryResult.action")?;
    let data = jmespath::Variable::from_json(json)?;
    let result = expr.search(data)?;

    match result.as_string() {
      Some(str_value) => return Ok(Variable::String(str_value.to_owned())),
      None => return Ok(Variable::Null)
    }
}

pub fn json_unwrap_string_value_or_panic (value: Variable) -> String {
  if let Variable::String(str_val) = value {
    str_val
  } else {
    panic!(format!("json_unwrap_string_value_or_panic failed for value {}", value ));
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
                  "Hi, this is Dummy Express, your specialist in international shipping."
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
    fn test_1() {
        let result = json_extract_value(JSON, "queryResult.action").unwrap();
        let value = json_unwrap_string_value_or_panic(result);
        assert_eq!("input.welcome", value);
    }
}
