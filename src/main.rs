use std::fs;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ctrlc;
use indicatif::{ProgressBar, ProgressStyle};
use yaml_rust::{Yaml, YamlLoader};

use gdf_testing::cmdl_parser::{get_cmd_line_parser, get_cmdl_options};
use gdf_testing::result_reporters::{HtmlResultReporter, JsonResultReporter, StdoutResultReporter};
use gdf_testing::suite_executor::TestSuiteExecutor;
use gdf_testing::thread_pool::ThreadPool;
use gdf_testing::yaml_parser::TestSuite;

// cargo run -- --suite-file c:/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/examples/sample_vap.yaml
// cargo run -- --suite-file c:/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/examples/sample_gdf.yaml
// cargo run -- --suite-file c:/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/examples/sample_vap.yaml --disable-stdout-report --html-report c:/tmp/report.html --json-report c:/tmp/report.json
fn main() {
    env_logger::init();
    let cmd_line_matches = get_cmd_line_parser().get_matches();
    let cmd_line_opts = get_cmdl_options(&cmd_line_matches);

    let test_suite_path = *cmd_line_opts.test_suite_file;
    let yaml_str = fs::read_to_string(test_suite_path);
    if let Err(some_err) = yaml_str {
        println!(
            "Error while reading yaml test suite definition file, terminating. Error detail: {}",
            some_err
        );
        process::exit(1);
    }
    let yaml_str = yaml_str.unwrap();

    // read the yaml file from string
    let docs = YamlLoader::load_from_str(&yaml_str);
    if let Err(some_err) = docs {
        println!(
            "Error while loading yaml test suite definition file, terminating. Error detail: {}",
            some_err
        );
        process::exit(1);
    }
    let docs: Vec<Yaml> = docs.unwrap();
    let yaml: &Yaml = &docs[0];

    //parse yaml string and convert it to test suite struct
    let suite = TestSuite::from_yaml(yaml);
    if let Err(some_err) = suite {
        println!(
            "Error while parsing yaml test suite definition file, terminating. Error detail: {}",
            some_err
        );
        process::exit(1);
    }
    let suite: TestSuite = suite.unwrap();

    // create test suite executor and underlying test executor jobs
    let suite_executor = TestSuiteExecutor::new(suite);
    if let Err(some_err) = suite_executor {
        println!(
            "Error while initiating the tests, terminating. Error detail: {}",
            some_err
        );
        process::exit(1);
    }
    let suite_executor = suite_executor.unwrap();

    // initiate thread pool for processing of test executor jobs
    let running = Arc::new(AtomicBool::new(true));
    let pool = ThreadPool::new(4, running.clone()); // TBD: make thread pool size configurable

    // initiate prohress bar for displaying execution progress
    let test_count = suite_executor.test_executors.len();
    let sty = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:70.yellow/red} {pos:>7}/{len:7} {msg}")
        .progress_chars("##-");

    let pb = ProgressBar::new(test_count as u64);
    pb.set_style(sty);

    // setup CTRL+C handler
    ctrlc::set_handler(move || {
        println!("CTRL+C detected!");
        running.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // kick off execution of all test executor jobs by thread pool
    for mut test_executor in suite_executor.test_executors {
        pool.execute(move || loop {
            let assertion_exec_result = test_executor.execute_next_assertion();
            if let None = assertion_exec_result {
                break;
            }
        });
    }

    // executed test with results will be returned by threadpool
    // via mpsc channel and gathered in this vector
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
            process::exit(1);
        }

        let executed_test = recv_res;

        if let Err(some_err) = executed_test {
            println!(
                "Error while running the tests, terminating. Error detail: {}",
                some_err
            );
            process::exit(1);
        }

        let executed_test = executed_test.unwrap();
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

    let print_to_std_out = cmd_line_opts.print_to_std_out;
    if print_to_std_out == true {
        println!("");
        StdoutResultReporter::report_test_results(&executed_tests);
        println!("");
    }

    let html_report_path = cmd_line_opts.html_report_path;
    if let Some(html_path) = html_report_path {
        let result = HtmlResultReporter::report_test_results(&executed_tests, *html_path);
        if let Err(some_error) = result {
            println!(
                "Error while generating html report. Error detail: {}",
                some_error
            );
            process::exit(1);
        }
    }

    let json_report_path = cmd_line_opts.json_report_path;
    if let Some(json_path) = json_report_path {
        let result = JsonResultReporter::report_test_results(&executed_tests, *json_path);
        if let Err(some_error) = result {
            println!(
                "Error while generating json report. Error detail: {}",
                some_error
            );
            process::exit(1);
        }
    }
}
