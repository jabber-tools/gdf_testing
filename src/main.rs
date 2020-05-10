use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ctrlc;
use indicatif::{ProgressBar, ProgressStyle};

use gdf_testing::result_reporters::{HtmlResultReporter, JsonResultReporter, StdoutResultReporter};
use gdf_testing::suite_executor::TestSuiteExecutor;
use gdf_testing::thread_pool::ThreadPool;
use gdf_testing::yaml_parser::TestSuite;
use yaml_rust::{Yaml, YamlLoader};

fn main() {
    #[allow(dead_code)]
    const YAML_STR_GDF: &str =
        "
        suite-spec:
            name: 'Dummy Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/src/testdata/credentials-cs-am-uat.json'
        tests:
            - name: 'Hello - track'
              desc: 'Simple initial two turn tracking dialog'
              lang: 'en'
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

    #[allow(dead_code)]
    const YAML_STR_VAP: &str = "
       suite-spec:
           name: 'Dummy Tracking'
           type: 'DHLVAP'
           config: 
             - vap_url: 'https://vap-dev.prg-dc.dhl.com:7070'
             - vap_access_token: '00b2018c-1a78-415c-8999-0852d503b1f3'
             - vap_svc_account_email: 'dummy-cs@iam.vap.dhl.com'
             - vap_svc_account_password: 'dummyPassword123'
       tests:
           - name: 'Hello - track'
             desc: 'Simple initial two turn tracking dialog'
             lang: 'es'
             assertions:
               - userSays: 'Hello'
                 botRespondsWith: 'Generic|BIT|0|Welcome|Gen'
               - userSays: 'track a package'
                 botRespondsWith: ['Tracking|CS|0|Prompt|Gen']
                 responseChecks:
                   - expression: 'dfResponse.queryResult.allRequiredParamsPresent'
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
                   - expression: 'dfResponse.queryResult.allRequiredParamsPresent'
                     operator: 'equals'
                     value: true
               - userSays: 'it is 1234567891'
                 botRespondsWith: ['Tracking|CS|4|Found OK|Gen']
                 responseChecks:
                   - expression: 'dfResponse.queryResult.action'
                     operator: 'equals'
                     value: 'express_track'
                   - expression: 'dfResponse.queryResult.parameters.tracking_id'
                     operator: 'equals'
                     value: '1234567891'
           - name: 'Human transfer'
             desc: 'Initiation of human transfer'
             assertions:
               - userSays: 'talk to representative'
                 botRespondsWith: 'Representative|CS|0|User request|Gen'
                 responseChecks:
                   - expression: 'dfResponse.queryResult.action'
                     operator: 'equals'
                     value: 'country_specific_response'                      
                   - expression: 'dfResponse.queryResult.parameters.event'
                     operator: 'equals'
                     value: 'repr_user_request'  
                   - expression: 'dfResponse.queryResult.allRequiredParamsPresent'
                     operator: 'equals'
                     value: true
      ";

    env_logger::init();

    let docs: Vec<Yaml> = YamlLoader::load_from_str(YAML_STR_GDF).unwrap();
    let yaml: &Yaml = &docs[0];
    let suite: TestSuite = TestSuite::from_yaml(yaml).unwrap();

    let suite_executor = TestSuiteExecutor::new(suite).unwrap();

    let running = Arc::new(AtomicBool::new(true));
    let pool = ThreadPool::new(4, running.clone()); // TBD: make thread pool size configurable

    let test_count = suite_executor.test_executors.len();

    let sty = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:70.yellow/red} {pos:>7}/{len:7} {msg}")
        .progress_chars("##-");

    let pb = ProgressBar::new(test_count as u64);
    pb.set_style(sty);

    ctrlc::set_handler(move || {
        println!("CTRL+C detected!");
        running.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    for mut test_executor in suite_executor.test_executors {
        pool.execute(move || loop {
            let assertion_exec_result = test_executor.execute_next_assertion();
            if let None = assertion_exec_result {
                break;
            }
        });
    }

    let mut executed_tests = vec![];

    println!("Runnint tests...");
    // by common sense we should start at zero but there is probably some bug
    // in indicatif library and it works properly only when we set it initually to 1
    pb.set_position(1);
    for i in 0..test_count
    /*lower bound inclusive, upper bound exclusive!*/
    {
        let recv_res = suite_executor.rx.recv();

        if let Err(_) = recv_res {
            println!("test results receiving channel broken, terminating.");
            break;
        }

        let executed_test = recv_res.unwrap();
        let test_result_str = StdoutResultReporter::get_test_result_str(&executed_test);
        pb.println(format!(
            "{} Finished test {} ({}/{})",
            test_result_str,
            executed_test.name,
            i + 1,
            test_count
        ));
        pb.inc(1);
        pb.set_message(&format!("Overall progress"));
        executed_tests.push(executed_test);
        // std::thread::sleep(std::time::Duration::from_millis(5000)); // just for nice progress bar debugging! remove from final code!
    }
    pb.finish_with_message("All tests executed!");

    println!("");
    StdoutResultReporter::report_test_results(&executed_tests);
    println!("");
    println!("");
    let _ = HtmlResultReporter::report_test_results(
        &executed_tests,
        Path::new("/tmp/sample_report.html"),
    );
    let _ = JsonResultReporter::report_test_results(
        &executed_tests,
        Path::new("/tmp/sample_report.json"),
    );
}
