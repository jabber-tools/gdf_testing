use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gdf_testing::errors::{Result, ErrorKind};
use yaml_rust::{YamlLoader, Yaml};
use gdf_testing::executor::TestSuiteExecutor;
use gdf_testing::thread_pool::ThreadPool;
use gdf_testing::yaml_parser::{
  Test,
  TestResult,
  TestAssertionResult, 
  TestSuite
};

#[macro_use] extern crate prettytable;
use prettytable::{Table, Row, Cell, Attr};
use indicatif::{ ProgressBar, ProgressStyle};
use ansi_term::Colour::{Red, Green, Yellow};
use ctrlc;

fn get_test_result_str_and_msg(test: &Test) -> (String, Option<String>) {
  let OK_STR = Green.paint("OK").to_string();
  let KO_STR = Red.paint("KO").to_string();
  let UNKNOWN_STR = Yellow.paint("??").to_string();

  let test_result_icon; 
  let mut test_err_msg: Option<String> = None;
  if let Some(test_result) = &test.test_result {
    match test_result {
      TestResult::Ok => {
        test_result_icon = OK_STR;
        test_err_msg = None;
      },
      _ => {
        test_result_icon = KO_STR;
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
    test_result_icon = UNKNOWN_STR; // this should never happen, but never say never :)
  }  
  (test_result_icon.to_string(), test_err_msg)
}

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

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let pool = ThreadPool::new(4, running.clone()); // for workers is good match for modern multi core PCs

    let test_count = suite_executor.test_executors.len();

    
    let sty = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .progress_chars("##-");    

    let pb = ProgressBar::new(test_count as u64);
    pb.set_style(sty);

    ctrlc::set_handler(move || {
        println!("CTRL+C detected!");
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");    


    for mut test_executor in suite_executor.test_executors {
        pool.execute(move || {
            while true {
                let assertion_exec_result = test_executor.execute_next_assertion();
                if let None =  assertion_exec_result {
                    break;
                }
            }             
        });
    }

    let mut executed_tests = vec![];

    println!("Runnint tests...");
    pb.set_position(1);
    for i in 0..test_count /*lower bound inclusive, upper bound exclusive!*/ { 

        let recv_res = suite_executor.rx.recv();

        if let Err(some_err) = recv_res {
          println!("test results receiving channel broken, terminating.");
          break;
        }

        let executed_test = recv_res.unwrap();
        let (test_result_str, test_err_msg) = get_test_result_str_and_msg(&executed_test);
        pb.println(format!("{} Finished test {} ({}/{})", test_result_str, executed_test.name, i + 1, test_count));
        pb.inc(1);    
        pb.set_message(&format!("Overall progress"));
        executed_tests.push(executed_test);
        // thread::sleep(Duration::from_millis(5000)); // just for nice progress bar debugging! remove from final code!
    }
    pb.finish_with_message("All tests executed!");

    println!("");

    let mut table = Table::new();
    table.add_row(row!["Test name", "Result", "Error message"]);

    for test in &executed_tests {
      let (test_result_str, test_err_msg) = get_test_result_str_and_msg(test);

      if let Some(err_msg) = test_err_msg {
        table.add_row(row![test.name, test_result_str, err_msg]);
      } else {
        table.add_row(row![test.name, test_result_str, ""]);
      }

    }

    table.printstd();
    println!(""); // without this table bottom row is not displayed


    // println!("");println!("");println!("");println!("");
    // println!("{:#?}", executed_tests);
}