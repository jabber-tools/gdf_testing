use gdf_testing::errors::{Result, ErrorKind};
use yaml_rust::{YamlLoader, Yaml};
use gdf_testing::executor::TestSuiteExecutor;
use gdf_testing::yaml_parser::{
  TestResult,
  TestAssertionResult, 
  TestSuite
};

use gdf_testing::thread_pool::ThreadPool;

use indicatif::ProgressBar;

#[macro_use] extern crate prettytable;
use prettytable::{Table, Row, Cell, Attr};

use std::thread;
use std::time::Duration;

#[allow(unused_must_use)]
fn main() {
        const YAML_STR: &str =
        "
        suite-spec:
            name: 'Dummy Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/src/testdata/credentials-cs-am-uat.json'
#            type: 'DHLVAP'
#            config: 
#              - vap_url: 'https://vap-dev.prg-dc.dhl.com:7070'
#              - vap_access_token: '00b2018c-1a78-415c-8999-0852d503b1f3'
#              - vap_svc_account_email: 'dummy-cs@iam.vap.dhl.com'
#              - vap_svc_account_password: 'dummyPassword123'
        tests:
            - name: 'Hello - track'
              desc: 'Simple initial two turn tracking dialog'
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: 'Generic|BIT|0|Welcome|Gen2'
                - userSays: 'track a package'
                  botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
                  responseChecks:
                    - expression: 'queryResult.allRequiredParamsPresent'
                      operator: 'equals'
                      value: true
            - name: 'Hello - track - entity parsing'
              desc: 'Very similar second test'
              assertions:
                - userSays: 'Hi'
                  botRespondsWith: 'Generic|BIT|0|Welcome|Gen2'
                - userSays: 'track a package please'
                  botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
                  responseChecks:
                    - expression: 'queryResult.allRequiredParamsPresent'
                      operator: 'equals'
                      value: true
                - userSays: 'it is 1234567891'
                  botRespondsWith: ['Tracking|CS|3|ID valid|Gen']
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: 'equals'
                      value: 'express_track'
                    - expression: 'queryResult.parameters.tracking_id'
                      operator: 'equals'
                      value: '1234567891'
            - name: 'Human transfer'
              desc: 'Initiation of human transfer'
              assertions:
                - userSays: 'talk to representative'
                  botRespondsWith: 'Representative|CS|0|User request|TPh'
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: 'equals'
                      value: 'country_specific_response'                      
                    - expression: 'queryResult.parameters.event'
                      operator: 'equals'
                      value: 'repr_user_request'  
                    - expression: 'queryResult.allRequiredParamsPresent'
                      operator: 'equals'
                      value: true
       ";         

    let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
    let yaml: &Yaml = &docs[0];
    let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    

    let mut suite_executor = TestSuiteExecutor::new(suite).unwrap();

    let pool = ThreadPool::new(4); // for workers is good match for modern multi core PCs

    let test_count = suite_executor.test_executors.len();

    let bar = ProgressBar::new(test_count as u64);

    for mut test_executor in suite_executor.test_executors {
        pool.execute(move || {
            while true {
                let assertion_exec_result = test_executor.execute_next_assertion();
                thread::sleep(Duration::from_millis(2000)); // just for nice progress bar debugging! remove from final code!
                if let None =  assertion_exec_result {
                    break;
                }
            }             
        });
    }

    let mut executed_tests = vec![];

    println!("Runnint tests...");
    for _ in 0..test_count {
        let executed_test = suite_executor.rx.recv().unwrap();
        
        /*let test_result = executed_test.get_test_error();

        if let Some(test_error_result) = test_result {
          
          match test_error_result {
            TestAssertionResult::KoIntentNameMismatch(err) => {
              println!("[{}] Intent name mismatch: {}", executed_test.name, err.message);
            },
            TestAssertionResult::KoResponseCheckError(err) => {
              println!("[{}] Assertion post check error: {}", executed_test.name, err.message);
            },
            _  => {/* this will never happen but Rust does not know that*/} 
          }

        } else {
          println!("[{}] no result!", executed_test.name);
        }*/
        bar.inc(1);
        executed_tests.push(executed_test);
    }


    println!("");

    let mut table = Table::new();
    table.add_row(row!["Test name", "Result", "Error message"]);

    for test in &executed_tests {
      let test_result_icon; // ✔ ❌ �
      let mut test_err_msg: Option<&str> = None;
      if let Some(test_result) = &test.test_result {
        match test_result {
          TestResult::Ok => {
            test_result_icon = "OK";
            test_err_msg = None;
          },
          _ => {
            test_result_icon = "KO";
            let test_error_result = &test.get_test_error().unwrap(); // quick and dirty ;)

            match test_error_result {
              TestAssertionResult::KoIntentNameMismatch(err) |
              TestAssertionResult::KoResponseCheckError(err) => {
                test_err_msg = Some(&err.message);
              },
              _  => {/* this will never happen but Rust does not know that*/}             
            }  
          }
        }
      } else {
        test_result_icon = "??"; // this should never happen, but never say never :)
      }

      if let Some(err_msg) = test_err_msg {
        table.add_row(row![test.name, test_result_icon, err_msg]);
      } else {
        table.add_row(row![test.name, test_result_icon, ""]);
      }

    }

    table.printstd();
    println!(""); // without this table bottom row is not displayed


    // println!("");println!("");println!("");println!("");
    // println!("{:#?}", executed_tests);
}