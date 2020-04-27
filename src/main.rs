use gdf_testing::errors::{Result, ErrorKind};
use yaml_rust::{YamlLoader, Yaml};
use gdf_testing::executor::TestSuiteExecutor;
use gdf_testing::yaml_parser::{
  TestResult, 
  TestSuite
};

use gdf_testing::thread_pool::ThreadPool;

use indicatif::ProgressBar;

#[macro_use] extern crate prettytable;
use prettytable::{Table, Row, Cell};

use std::thread;
use std::time::Duration;

#[allow(unused_must_use)]
fn main() -> Result<()> {
        const YAML_STR: &str =
        "
        suite-spec:
            name: 'Dummy Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/src/testdata/credentials.json'
        tests:
            - name: 'Hello - track'
              desc: 'Simple initial two turn tracking dialog'
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
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
                  botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
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

    let mut suite_executor = TestSuiteExecutor::new(suite)?;

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
        let test_result = suite_executor.rx.recv().unwrap();
        executed_tests.push(test_result);
        bar.inc(1);
        
    }

    println!("Tests results below:");

    for test in &executed_tests {
      let test_result_icon; // ✔ ❌ �
      if let Some(test_result) = &test.test_result {
        match test_result {
          TestResult::Ok => test_result_icon = "OK",
          _ => test_result_icon = "KO"
        }
      } else {
        test_result_icon = "??"; // this should never happen, but never say never :)
      }

      println!("{} [{}] ", test_result_icon, test.name);
      
    }

    println!(""); // without this last println from previous loop is not displayed!

    // println!("");println!("");println!("");println!("");
    // println!("{:#?}", executed_tests);

    let mut table = Table::new();
    table.add_row(row!["Test name", "Error message"]);
    for test in &executed_tests {
      let test_to_str = format!("{:#?}", &test);
      // table.add_row(row![test.name, &test_to_str]);
      table.add_row(row![test.name, "TBD..."]);
    }
    table.printstd();
    println!(""); // without this table bottom row is not displayed

    Ok(())
}