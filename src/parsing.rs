pub mod yaml {

    use yaml_rust::YamlLoader;
    use yaml_rust::Yaml;
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    struct YamlParsingError(String);

    impl fmt::Display for YamlParsingError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "An Error Occurred, Please Try Again!")
        }
    }

    // must be implemented so that we 
    // can return Err(Box::new(YamlParsingError(format!("Unknown suite type"))));
    impl Error for YamlParsingError {}    


    enum TestSuiteType {
        DialogFlow,
        DHLVAP
    }

    struct TestSuiteSpec {
        name: String,
        suite_type: TestSuiteType,
        cred: String
    }

    impl TestSuiteSpec {
        fn new(name: String, suite_type: TestSuiteType, cred: String) -> TestSuiteSpec {
            TestSuiteSpec {
                name,
                suite_type,
                cred
            }
        }
    }

    struct TestAssertion {
        user_says: String,
        bot_responds_with: Vec<String>
    }

    impl TestAssertion {
        pub fn new(user_says: String) -> Self {
            TestAssertion {
                user_says: user_says,
                bot_responds_with: vec![]
            }
        }
    }

    struct Test {
        name: String,
        desc: Option<String>,
        assertions: Vec<TestAssertion>
    }
    
    impl Test {
        pub fn new(name: String, desc: Option<String>) -> Self {
            Test {
                name: name,
                desc: desc,
                assertions: vec![]
            }
        }

        pub fn set_assertions(&mut self, mut assertions: Vec<TestAssertion>) {
            &self.assertions.append(&mut assertions);
        }
    }

    pub struct TestSuite {
        suite_spec: TestSuiteSpec,
        tests: Vec<Test>
    }

    impl TestSuite {
        pub fn new(yaml: &str) -> Result<TestSuite, Box<dyn Error>> {
            let docs = YamlLoader::load_from_str(yaml)?;
            let yaml: &yaml_rust::yaml::Yaml = &docs[0];

            let name: Option<&str> = yaml["suite-spec"]["name"].as_str();
            if let None = name {
                return Err(Box::new(YamlParsingError(format!("Suite name not specified"))));
            }

            let suite_type: Option<&str> = yaml["suite-spec"]["type"].as_str();
            let suite_type: Option<TestSuiteType> = match suite_type {
                Some("DialogFlow") => Some(TestSuiteType::DialogFlow),
                Some("DHLVAP") => Some(TestSuiteType::DHLVAP),
                Some(unknown) =>  return Err(Box::new(YamlParsingError(format!("Unknown suite type found: {}", unknown)))),
                None => return Err(Box::new(YamlParsingError(String::from("Suite type not specified"))))
            };

            let cred: Option<&str> = yaml["suite-spec"]["cred"].as_str();
            if let None = cred {
                return Err(Box::new(YamlParsingError(format!("Suite credentials not specified"))));
            }
            
            let tests = yaml["tests"].as_vec();
            if let None = tests {
                return Err(Box::new(YamlParsingError(format!("No tests specified"))));
            }

            let tests = tests.unwrap();
            println!("number of tests {}", tests.len());

            if tests.len() == 0 {
                return Err(Box::new(YamlParsingError(format!("No tests specified"))));
            }

            let mut suite_tests: Vec<Test> = vec![];

            for test in tests.iter() {

                let test_name = test["name"].as_str();
                let _test_desc = test["desc"].as_str(); //desc is optional
                let test_assertions = test["assertions"].as_vec();
                if let None = test_name {return Err(Box::new(YamlParsingError(format!("Test name not specified"))));}
                
                let test_to_push;

                if let None = test_assertions {
                    test_to_push = Test::new(test_name.unwrap().to_owned(), None);
                    suite_tests.push(test_to_push);
                } else {
                    test_to_push = Test::new(test_name.unwrap().to_owned(), Some(_test_desc.unwrap().to_string()));
                    suite_tests.push(test_to_push);
                }

                

                let mut assertions_found = true;
                if let None = test_assertions { assertions_found = false; }
                let test_assertions = test_assertions.unwrap();
                if test_assertions.len() == 0 { 
                    assertions_found = false; 
                }

                if assertions_found == false {
                    return Err(Box::new(YamlParsingError(format!("Test assertions not specified for {}", test_name.unwrap()))));
                }

                
                let test_assertions_to_push: Vec<TestAssertion> = vec![];

                for test_assertion in test_assertions.iter() {
                    let user_says = test_assertion["userSays"].as_str();
                    if let None = user_says {
                        return Err(Box::new(YamlParsingError(format!("Test assertions missing userSays for {}", test_name.unwrap()))));
                    }
                    let user_says = user_says.unwrap();

                    let bot_responds_with = test_assertion["botRespondsWith"].as_str();
                    if let None = bot_responds_with {
                        let bot_responds_with = test_assertion["botRespondsWith"].as_vec();
                        if let None = bot_responds_with {
                            return Err(Box::new(YamlParsingError(format!("Test assertions missing botRespondsWith for {}", test_name.unwrap()))));
                        } else {
                            let bot_responds_with = bot_responds_with.unwrap();

                            for bot_responds_with_str in bot_responds_with.iter() {
                                println!("bot_responds_with_str {}", bot_responds_with_str.as_str().unwrap());
                            }

                        }
                    } else {
                        let bot_responds_with = bot_responds_with.unwrap();
                        println!("xixi {:?}", bot_responds_with);
                    }
                }

                // test_to_push.set_assertions(test_assertions_to_push);

            }

            Ok(
                TestSuite {
                    // we can safely unwrap now, None value is not possible here
                    suite_spec: TestSuiteSpec::new(name.unwrap().to_string(), suite_type.unwrap(), cred.unwrap().to_string()),
                    tests: vec![]
            }) 
        }

        pub fn get_suite_name (&self) -> &str {
            &self.suite_spec.name.as_str()
        }
    }

    pub fn parse (yaml: &str) {
        TestSuite::new(yaml);
    }
}

pub mod json {
    pub fn parse () {
        println!("yaml::json");
    }
}

#[cfg(test)]
mod tests {
    use super::yaml::*;

const YAML1: &str =
"
suite-spec:
    name: 'Express Tracking'
    type: 'DialogFlow'
    cred: '/path/to/cred'
tests:
    - name: 'Welcome intent test'
      desc: 'Tests default welcome intent'
      assertions:
        - userSays: 'Hello'
          botRespondsWith: ['Welcome']
    - name: 'Default fallback intent'
      desc: 'Tests default fallback intent'
      assertions:
        - userSays: 'wtf'
          botRespondsWith: ['Fallback']
";            



    #[test]
    fn test_parse () {
        let suite =  TestSuite::new(YAML1);
        assert_eq!(suite.unwrap().get_suite_name(), "Express Tracking");
        parse(YAML1);
    }
}