use prettytable::{row, cell}; // macros
use prettytable::{Table};
use ansi_term::Colour::{Red, Green, Yellow};

use crate::yaml_parser::{
    Test,
    TestResult,
    TestAssertionResult, 
};

pub fn get_test_result_str_and_msg(test: &Test) -> (String, Option<TestAssertionResult>) {
    let ok_str = Green.paint("OK").to_string();
    let ko_str = Red.paint("KO").to_string();
    let unknown_str = Yellow.paint("??").to_string();
  
    let test_result_icon; 
    let mut test_err_result: Option<TestAssertionResult> = None;
    if let Some(test_result) = &test.test_result {
      match test_result {
        TestResult::Ok => {
          test_result_icon = ok_str;
          test_err_result = None;
        },
        _ => {
          test_result_icon = ko_str;
          let test_error_result = test.get_test_error().unwrap(); // quick and dirty ;)
  
          match test_error_result {
            TestAssertionResult::KoIntentNameMismatch(_) |
            TestAssertionResult::KoResponseCheckError(_) => {
              test_err_result = Some(test_error_result.clone());
            },
            _  => { /* this will never happen but Rust does not know that */ }             
          }  
        }
      }
    } else {
      test_result_icon = unknown_str; // this should never happen, but never say never :)
      test_err_result = None;
    }

    (test_result_icon.to_string(), test_err_result)
}


pub fn print_test_summary_table(executed_tests: &Vec<Test>) {

  let mut table = Table::new();
  table.add_row(row!["Test name", "Result", "Error message"]);

  for test in executed_tests {
    let (test_result_str, test_err_result_unwraped) = get_test_result_str_and_msg(test);

    if let Some(test_err_result) = test_err_result_unwraped {
      match test_err_result {
        TestAssertionResult::KoIntentNameMismatch(err) => {
          table.add_row(row![test.name, test_result_str, "Intent name mismatch:\n".to_owned() + &err.message]);
        },
        TestAssertionResult::KoResponseCheckError(err) => {
          table.add_row(row![test.name, test_result_str, "Assertion post check error:\n".to_owned() + &err.message]);
        },
        TestAssertionResult::Ok(_) => { 
          /* will not happen but rust does not know that */
        }
      }
    } else {
      table.add_row(row![test.name, test_result_str, ""]); 
    }
  } 
  table.printstd();
}

#[cfg(test)]
mod tests {
  use super::*;

  #[allow(dead_code)]
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

  // cargo test -- --show-output test_result_table
  #[test]
  #[ignore]
  fn test_result_table() {

    let ok_str = Green.paint("OK").to_string();
    let ko_str = Red.paint("KO").to_string();
    let na_str = Yellow.paint("N/A").to_string();

    let mut table_assertion1_postchecks = Table::new();
    table_assertion1_postchecks.add_row(row!["Expression", "Operator", "Value", "Status"]);
    table_assertion1_postchecks.add_row(row!["queryResult.allRequiredParamsPresent", "=", "true", ok_str]);
    table_assertion1_postchecks.add_row(row!["queryResult.allRequiredParamsPresent", "=", "true", ok_str]);

    let mut table_assertion2_postchecks = Table::new();
    table_assertion2_postchecks.add_row(row!["Expression", "Operator", "Value", "Status"]);
    table_assertion2_postchecks.add_row(row!["queryResult.allRequiredParamsPresent", "=", "true", ok_str]);
    table_assertion2_postchecks.add_row(row!["queryResult.action", "=", "express_track", ko_str]);


    let mut table_assertion1 = Table::new();
    table_assertion1.add_row(row!["Usar says", "Bot responds with", "Status", "Raw response", "Post Checks"]);
    table_assertion1.add_row(row!["Hello", "Generic|BIT|0|Welcome|Gen", ok_str, "{'foo':'bar'}", table_assertion1_postchecks]);
    table_assertion1.add_row(row!["Hello", "Generic|BIT|0|Welcome|Gen", ko_str, "{'foo':'bar'}", na_str]);

    let mut table_assertion2 = Table::new();
    table_assertion2.add_row(row!["Usar says", "Bot responds with", "Status", "Raw response", "Post Checks"]);
    table_assertion2.add_row(row!["Hello", "Generic|BIT|0|Welcome|Gen", ok_str, "{'foo':'bar'}", table_assertion2_postchecks]);
    table_assertion2.add_row(row!["Hello", "Generic|BIT|0|Welcome|Gen", ko_str, "{'foo':'bar'}", na_str]);


    let mut table_test = Table::new();
    table_test.add_row(row!["Test1"]);
    table_test.add_row(row![table_assertion1]);
    table_test.add_row(row!["Test2"]);
    table_test.add_row(row![table_assertion2]);

    table_test.printstd();
  }
}

