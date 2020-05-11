use clap::{App, Arg, ArgMatches};
use log::debug;
use std::path::Path;

pub struct CommandLine<'a> {
    pub test_suite_file: Box<&'a Path>,
    pub print_to_std_out: bool,
    pub html_report_path: Option<Box<&'a Path>>,
    pub json_report_path: Option<Box<&'a Path>>,
    pub threadpool_size: usize,
}

impl<'a> CommandLine<'a> {
    fn new(test_suite_file: Box<&'a Path>) -> Self {
        return CommandLine {
            test_suite_file,
            print_to_std_out: true,
            html_report_path: None,
            json_report_path: None,
            threadpool_size: 4,
        };
    }
}

pub fn get_cmd_line_parser<'a, 'b>() -> App<'a, 'b> {
    App::new("Google DialogFlow Testing")
        .version("0.1.0")
        .author("Adam Bezecny")
        .about("Tool for automated testing of chatbots based on Google DialogFlow NLP")
        .arg(
            Arg::with_name("suite_file")
                .short("f")
                .long("suite-file")
                .value_name("FILE")
                .help("Yaml file with test suite definition")
                .takes_value(true)
                .required(true)
        )
        .arg(
            Arg::with_name("html_report")
                .long("html-report")
                .value_name("FILE")
                .help("Path to optional html report")
                .takes_value(true)
                .required(false)
        )
        .arg(
            Arg::with_name("json_report")
                .long("json-report")
                .value_name("FILE")
                .help("Path to optional json report")
                .takes_value(true)
                .required(false)
        )
        .arg(
            Arg::with_name("surpress_stdout_report")
                .long("disable-stdout-report")
                .help("Disables default report printed to standard output")
                .required(false)
        )
        .arg(
            Arg::with_name("threadpool_size")
                .short("t")
                .long("threadpool-size")
                .value_name("INTEGER")
                .help("Number of worker threads for parallel test execution. If not specified defaults to 4.")
                .takes_value(true)
                .default_value("4")
        )
}

pub fn get_cmdl_options<'a>(matches: &'a ArgMatches) -> CommandLine<'a> {
    let mut command_line;
    if let Some(file) = matches.value_of("suite_file") {
        debug!("Value for suite_file: {}", file);
        command_line = CommandLine::new(Box::new(Path::new(file)));
    } else {
        // this will never hapen since clap will not allow to get here without suite file
        // but we need to implement this to fool compiler, otherwise it will be complaining about error:
        // use of possibly-uninitialized `command_line`
        debug!("suite file not specified, terminating.");
        std::process::exit(1);
    }

    if let Some(file) = matches.value_of("html_report") {
        debug!("Value for html_report: {}", file);
        command_line.html_report_path = Some(Box::new(Path::new(file)));
    }

    if let Some(file) = matches.value_of("json_report") {
        debug!("Value for json_report: {}", file);
        command_line.json_report_path = Some(Box::new(Path::new(file)));
    }

    if matches.is_present("surpress_stdout_report") {
        debug!("Standard output report will be surpressed.");
        command_line.print_to_std_out = false;
    }

    // safe to unwrap, clap provides default value
    command_line.threadpool_size = matches
        .value_of("threadpool_size")
        .unwrap()
        .to_owned()
        .parse::<usize>()
        .unwrap();

    command_line
}
