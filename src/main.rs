use gdf_testing::errors::{Result, ErrorKind};
use yaml_rust::{YamlLoader, Yaml};
use gdf_testing::executor::TestSuiteExecutor;
use gdf_testing::yaml_parser::TestSuite;
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

    let mut suite_executor = TestSuiteExecutor::new(suite)?;

    let pool = ThreadPool::new(4); // for workers is good match for modern multi core PCs

    let res_count = suite_executor.test_executors.len();

    for mut test_executor in suite_executor.test_executors {
        pool.execute(move || {
            while true {
                println!();
                let assertion_exec_result = test_executor.execute_next_assertion();
                if let None =  assertion_exec_result {
                    break;
                }
            }             
        });
    }

    for _ in 0..res_count {
        let test_result = suite_executor.rx.recv().unwrap();
        println!("test result {:#?}", test_result);
    }

    println!("terminating main with Ok(())");
    Ok(())
}