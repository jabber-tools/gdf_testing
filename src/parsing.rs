pub mod yaml {

    use yaml_rust::YamlLoader;
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

    struct Test {
        name: String,
        desc: String,
        assertions: Vec<TestAssertion>
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
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        let doc = &docs[0];
        println!("yaml::parse {}", doc["suite-spec"]["name"].as_str().unwrap());
        println!("yaml::parse {}", doc["tests"][0]["name"].as_str().unwrap());
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
";            



    #[test]
    fn test_parse () {
        let suite =  TestSuite::new(YAML1);
        assert_eq!(suite.unwrap().get_suite_name(), "Express Tracking");
        parse(YAML1);
    }
}