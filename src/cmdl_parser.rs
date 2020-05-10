use clap::{App, Arg, ArgMatches};

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
                .required(false), // TBD: this will be required soon!
        )
}

pub fn check_cmdl_matches(matches: &ArgMatches) {
    if let Some(file) = matches.value_of("suite_file") {
        println!("Value for suite_file: {}", file);
    }
}
