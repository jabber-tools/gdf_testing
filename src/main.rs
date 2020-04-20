// use gdf_testing;
// use std::error::Error;
// use yaml_rust::YamlLoader;
use gdf_testing::errors::{Result, ErrorKind, Error};
use yaml_rust::{YamlLoader, Yaml};
use gdf_testing::executor::{TestExecutor, TestSuiteExecutor};
use gdf_testing::yaml_parser::{
  Test, 
  TestAssertion, 
  TestSuiteType, 
  TestSuite, 
  TestAssertionResponseCheckOperator,
  TestAssertionResponseCheckValue,
  TestAssertionResponseCheck
};
use gdf_testing::thread_pool::ThreadPool;

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
      - name: 'Hello - track - 2'
        desc: 'Very similar second test'
        assertions:
          - userSays: 'Hello'
            botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
          - userSays: 'how do I change my address'
            botRespondsWith: ['FAQ|CS|0|Address change|TPh']
            responseChecks:
            - expression: 'queryResult.action'
              operator: 'equals'
              value: 'country_specific_response'
            - expression: 'queryResult.parameters.event'
              operator: 'equals'
              value: 'faq_address_change'
 ";           

  let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR).unwrap();
  let yaml: &Yaml = &docs[0];
  let suite: TestSuite =  TestSuite::from_yaml(yaml).unwrap();    

  let mut suite_executor = TestSuiteExecutor::new(&suite)?;

  let pool = ThreadPool::new(4); // for workers is good match for modern multi core PCs

  /*for test_executor in &mut suite_executor.test_executors {
      pool.execute(|| {
  
          while true {
              println!();
              let details_result = test_executor.next_assertion_details();
  
              if let None = details_result {
                  println!("all assertions processed!");
                  break; // all asertions were processed -> break
              }
  
              let user_says = &details_result.unwrap().user_says;
  
              print!("Saying {}", user_says);
              let assertion_result = test_executor.execute_next_assertion().unwrap();
  
              if let Err(err) =  assertion_result {
                  match *err.kind {
                      ErrorKind::InvalidTestAssertionEvaluation => {
                          print!(" - ko! {}", err.message);
                      },
                      _ =>  print!(" - ko! {}", err)
                  }
              } else {
                  print!(" - ok!");
              }
          }             
          
      });
  }*/


  Ok(())
}