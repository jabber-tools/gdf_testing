use ansi_term::Colour::{Green, Red, Yellow};
use prettytable::Table;
use prettytable::{cell, row}; // macros

use crate::yaml_parser::{Test, TestAssertionResult, TestResult};

pub struct StdoutResultReporter;

impl StdoutResultReporter {
    fn get_ok_str() -> String {
        Green.paint("OK").to_string()
    }

    fn get_na_str() -> String {
        Green.paint("N/A").to_string()
    }

    fn get_ko_str() -> String {
        Red.paint("KO").to_string()
    }

    fn get_unknown_str() -> String {
        Yellow.paint("??").to_string()
    }

    fn get_not_executed_str() -> String {
        Yellow.paint("Not executed").to_string()
    }

    pub fn get_test_result_str(test: &Test) -> String {
        let test_result_str;
        if let Some(test_result) = &test.test_result {
            match test_result {
                TestResult::Ok => test_result_str = StdoutResultReporter::get_ok_str(),
                TestResult::Ko => test_result_str = StdoutResultReporter::get_ko_str(),
            }
        } else {
            test_result_str = StdoutResultReporter::get_unknown_str();
        }

        test_result_str
    }

    pub fn report_test_results(tests: &Vec<Test>) {
        let mut test_tables: Vec<Table> = vec![];

        for test in tests {
            let mut test_table = Table::new();
            let test_result_str = StdoutResultReporter::get_test_result_str(test);

            let test_result = test.get_test_error();

            // add header row with test name status string (OK/KO) + potential error message (either intent name mismatch or assertion check error)
            match test_result {
                Some(some_test_result) => {
                    match some_test_result {
                        TestAssertionResult::KoIntentNameMismatch(err) => {
                            test_table.add_row(row![
                                test.name.clone() + " - " + &test_result_str + "\n" + &err.message
                            ]);
                        }
                        TestAssertionResult::KoResponseCheckError(err, _) => {
                            test_table.add_row(row![
                                test.name.clone() + " - " + &test_result_str + "\n" + &err.message
                            ]);
                        }
                        _ => { /* ok will not happen get_test_error is returning none in that case */
                        }
                    }
                }
                None => {
                    test_table.add_row(row![test.name.clone() + " - " + &test_result_str]);
                }
            } // match test_result

            // now add assertion table within second row of master table (test_table)
            let mut test_table_assertions = Table::new();
            test_table_assertions.add_row(row![
                "User says",
                "Bot responds with",
                "Intent match status",
                "Assertion checks",
                "Raw response"
            ]);
            for assertion in &test.assertions {
                match assertion.test_assertion_result.as_ref().unwrap() /* assuming we always have result! */ {
          TestAssertionResult::Ok(_) => {
            test_table_assertions.add_row(
              row![
                assertion.user_says.clone(),
                assertion.bot_responds_with.join("\n"),
                StdoutResultReporter::get_ok_str(),
                match assertion.response_checks.len() {
                  0 => StdoutResultReporter::get_na_str(),
                  _ => StdoutResultReporter::get_ok_str()
                },
                "" // if everything is OK do not include backed response in std out report,
                   // it will be collapsed in html report
                   // TBD: other option is to make this configurable
              ]
            );
          },
          TestAssertionResult::KoIntentNameMismatch(err) => {
            test_table_assertions.add_row(
              row![
                assertion.user_says.clone(),
                assertion.bot_responds_with.join("\n"), 
                StdoutResultReporter::get_ko_str(),
                StdoutResultReporter::get_not_executed_str(),
                err.backend_response.as_ref().unwrap() // TBD: make this configurable!
              ]
            );
            break; // do not continue with any other assertion!
          },
          TestAssertionResult::KoResponseCheckError(err, assertion_check_idx) => {

            let mut test_table_assertion_resp_checks = Table::new();
            test_table_assertion_resp_checks.add_row(row!["Expression", "Operator", "Value", "Status"]);

            for idx in 0..*assertion_check_idx + 1 {
              let response_check = &assertion.response_checks[idx];

              let res_str;
              if idx == *assertion_check_idx {
                res_str = StdoutResultReporter::get_ko_str()
              } else {
                res_str = StdoutResultReporter::get_ok_str()
              }

              test_table_assertion_resp_checks.add_row(
                row![
                  response_check.expression,
                  response_check.operator,
                  response_check.value,
                  res_str
                ]
              );
            }

            test_table_assertions.add_row(
              row![
                assertion.user_says.clone(),
                assertion.bot_responds_with.join("\n"), 
                StdoutResultReporter::get_ok_str(),
                test_table_assertion_resp_checks,
                err.backend_response.as_ref().unwrap() // TBD: make this configurable!
              ]

            );
            break; // do not continue with any other assertion!
          },
        }
            } // for assertion in test.assertions
            test_table.add_row(row![test_table_assertions]);
            test_tables.push(test_table);
        } // for test in tests

        for table in test_tables {
            table.printstd();
        }
    } // report_test_results
} // impl StdoutResultReporter

#[cfg(test)]
mod tests {
    use super::*;

    const JSON_FOO: &str = r#"
  {
      "foo": "bar",
  }
  "#;

    #[allow(dead_code)]
    const JSON: &str = r#"
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
        let na_str = "N/A".to_string();
        let ne_str = Yellow.paint("Not executed").to_string();

        let mut table_assertion1_postchecks = Table::new();
        table_assertion1_postchecks.add_row(row!["Expression", "Operator", "Value", "Status"]);
        table_assertion1_postchecks.add_row(row![
            "queryResult.allRequiredParamsPresent",
            "=",
            "true",
            ok_str
        ]);
        table_assertion1_postchecks.add_row(row![
            "queryResult.allRequiredParamsPresent",
            "=",
            "true",
            ok_str
        ]);

        let mut table_assertion2_postchecks = Table::new();
        table_assertion2_postchecks.add_row(row!["Expression", "Operator", "Value", "Status"]);
        table_assertion2_postchecks.add_row(row![
            "queryResult.allRequiredParamsPresent",
            "=",
            "true",
            ok_str
        ]);
        table_assertion2_postchecks.add_row(row![
            "queryResult.allRequiredParamsPresent",
            "=",
            "true",
            ko_str
        ]);

        let mut table_assertion2_postchecks = Table::new();
        table_assertion2_postchecks.add_row(row!["Expression", "Operator", "Value", "Status"]);
        table_assertion2_postchecks.add_row(row![
            "queryResult.allRequiredParamsPresent",
            "=",
            "true",
            ok_str
        ]);
        table_assertion2_postchecks.add_row(row![
            "queryResult.action",
            "=",
            "express_track",
            ko_str
        ]);

        let mut table_assertion1 = Table::new();
        table_assertion1.add_row(row!["Test assertions"]);
        table_assertion1.add_row(row![
            "User says",
            "Bot responds with",
            "Intent match status",
            "Assertion checks",
            "Raw response"
        ]);
        table_assertion1.add_row(row![
            "Hello",
            "Generic|BIT|0|Welcome|Gen",
            ok_str,
            na_str,
            "{'foo':'bar'}"
        ]);
        table_assertion1.add_row(row![
            "Hello",
            "Generic|BIT|0|Welcome|Gen",
            ok_str,
            table_assertion1_postchecks,
            JSON_FOO
        ]);

        let mut table_assertion2 = Table::new();
        table_assertion2.add_row(row!["Test assertions"]);
        table_assertion2.add_row(row![
            "User says",
            "Bot responds with",
            "Intent match status",
            "Assertion checks",
            "Raw response"
        ]);
        table_assertion2.add_row(row![
            "Hello",
            "Generic|BIT|0|Welcome|Gen",
            ok_str,
            na_str,
            JSON_FOO
        ]);
        table_assertion2.add_row(row![
            "Hello",
            "Generic|BIT|0|Welcome|Gen",
            ko_str,
            ne_str,
            JSON_FOO
        ]);

        let mut table_assertion3 = Table::new();
        table_assertion3.add_row(row!["Test assertions"]);
        table_assertion3.add_row(row![
            "User says",
            "Bot responds with",
            "Intent match status",
            "Assertion checks",
            "Raw response"
        ]);
        table_assertion3.add_row(row![
            "Hello",
            "Generic|BIT|0|Welcome|Gen",
            ok_str,
            na_str,
            JSON_FOO
        ]);
        // table_assertion3.add_row(row!["Hello", "Generic|BIT|0|Welcome|Gen", ok_str, table_assertion2_postchecks, JSON]);
        table_assertion3.add_row(row![
            "Hello",
            "Generic|BIT|0|Welcome|Gen",
            ok_str,
            table_assertion2_postchecks,
            JSON_FOO
        ]);

        let mut table_test1 = Table::new();
        table_test1.add_row(row!["Test1 - ".to_owned() + &ok_str]);
        table_test1.add_row(row![table_assertion1]);

        let mut table_test2 = Table::new();
        table_test2.add_row(row!["Test2 - ".to_owned() + &ko_str + "\n" + "Wrong intent name received. Expected one of: 'Generic|BIT|0|Welcome|Gen', got: 'Generic|BIT|0|Welcome|Gen123'"]);
        table_test2.add_row(row![table_assertion2]);

        let mut table_test3 = Table::new();
        table_test3.add_row(row!["Test3 - ".to_owned() + &ko_str + "\n" + "Expected value 'express_track' does not match real value: 'express_track123' for expression: queryResult.action"]);
        table_test3.add_row(row![table_assertion3]);

        table_test1.printstd();
        println!("");
        println!("");
        table_test2.printstd();
        println!("");
        println!("");
        table_test3.printstd();
    }
}
