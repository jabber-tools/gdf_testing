use prettytable::{row, cell}; // macros
use prettytable::{Table};
use ansi_term::Colour::{Red, Green, Yellow};

use crate::yaml_parser::{
    Test,
    TestResult,
    TestAssertionResult, 
};

pub fn get_test_result_str_and_msg(test: &Test) -> (String, Option<String>) {
    let ok_str = Green.paint("OK").to_string();
    let ko_str = Red.paint("KO").to_string();
    let unknown_str = Yellow.paint("??").to_string();
  
    let test_result_icon; 
    let mut test_err_msg: Option<String> = None;
    if let Some(test_result) = &test.test_result {
      match test_result {
        TestResult::Ok => {
          test_result_icon = ok_str;
          test_err_msg = None;
        },
        _ => {
          test_result_icon = ko_str;
          let test_error_result = test.get_test_error().unwrap(); // quick and dirty ;)
  
          match test_error_result {
            TestAssertionResult::KoIntentNameMismatch(err) |
            TestAssertionResult::KoResponseCheckError(err) => {
              test_err_msg = Some(err.message.to_owned());
            },
            _  => {/* this will never happen but Rust does not know that*/}             
          }  
        }
      }
    } else {
      test_result_icon = unknown_str; // this should never happen, but never say never :)
    }  
    (test_result_icon.to_string(), test_err_msg)
  }

  pub fn print_test_summary_table(executed_tests: &Vec<Test>) {

    let mut table = Table::new();
    table.add_row(row!["Test name", "Result", "Error message"]);

    for test in executed_tests {
      let (test_result_str, test_err_msg) = get_test_result_str_and_msg(test);

      if let Some(err_msg) = test_err_msg {
        table.add_row(row![test.name, test_result_str, err_msg]);
      } else {
        table.add_row(row![test.name, test_result_str, ""]);
      }

    }

    table.printstd();
  }