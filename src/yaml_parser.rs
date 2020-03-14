use yaml_rust::YamlLoader;
use yaml_rust::Yaml;
use std::fmt;
use yaml_rust::scanner::ScanError;
use std::error::Error;

#[derive(Debug)]
pub struct YamlParsingError(String);

impl fmt::Display for YamlParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "An Error Occurred, Please Try Again!")
    }
}

impl From<ScanError> for YamlParsingError {
    fn from(error: ScanError) -> Self {
        YamlParsingError(format!("error when parsing yaml: {}", error))
    }    
}

impl Error for YamlParsingError {}

#[derive(Debug)]
pub enum TestSuiteType {
    DialogFlow,
    DHLVAP
}

#[derive(Debug)]
pub struct TestSuiteSpec {
    pub name: String,
    pub suite_type: TestSuiteType,
    pub cred: String
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

#[derive(Debug)]
pub struct TestAssertion {
    pub user_says: String,
    pub bot_responds_with: Vec<String>
}

impl TestAssertion {
    pub fn new(user_says: String, bot_responds_with: Vec<String>) -> Self {
        TestAssertion {
            user_says: user_says,
            bot_responds_with: bot_responds_with
        }
    }
}

#[derive(Debug)]
pub struct Test {
    pub name: String,
    pub desc: Option<String>,
    pub assertions: Vec<TestAssertion>
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

#[derive(Debug)]
pub struct TestSuite {
    pub suite_spec: TestSuiteSpec,
    pub tests: Vec<Test>
}

impl TestSuite {
    pub fn new(yaml: &str) -> Result<TestSuite, YamlParsingError> {
        let docs = YamlLoader::load_from_str(yaml)?;
        let yaml: &Yaml = &docs[0];

        let name: Option<&str> = yaml["suite-spec"]["name"].as_str();
        if let None = name {
            return Err(YamlParsingError(format!("Suite name not specified")));
        }

        let suite_type: Option<&str> = yaml["suite-spec"]["type"].as_str();
        let suite_type: Option<TestSuiteType> = match suite_type {
            Some("DialogFlow") => Some(TestSuiteType::DialogFlow),
            Some("DHLVAP") => Some(TestSuiteType::DHLVAP),
            Some(unknown) =>  return Err(YamlParsingError(format!("Unknown suite type found: {}", unknown))),
            None => return Err(YamlParsingError(String::from("Suite type not specified")))
        };

        let cred: Option<&str> = yaml["suite-spec"]["cred"].as_str();
        if let None = cred {
            return Err(YamlParsingError(format!("Suite credentials not specified")));
        }
            
        let tests = yaml["tests"].as_vec();
        if let None = tests {
            return Err(YamlParsingError(format!("No tests specified")));
        }

        let tests = tests.unwrap();
        println!("number of tests {}", tests.len());

        if tests.len() == 0 {
            return Err(YamlParsingError(format!("No tests specified")));
        }

        let mut suite_tests: Vec<Test> = vec![];

        for test in tests.iter() {

            let test_name = test["name"].as_str();
            let _test_desc = test["desc"].as_str(); //desc is optional
            let test_assertions = test["assertions"].as_vec();
            if let None = test_name {return Err(YamlParsingError(format!("Test name not specified")));}
                
            let mut test_to_push;

            if let None = test_assertions {
                test_to_push = Test::new(test_name.unwrap().to_owned(), None);
            } else {
                test_to_push = Test::new(test_name.unwrap().to_owned(), Some(_test_desc.unwrap().to_string()));
            }
                

            let mut assertions_found = true;
            if let None = test_assertions { assertions_found = false; }
            let test_assertions = test_assertions.unwrap();
            if test_assertions.len() == 0 { 
                assertions_found = false; 
            }

            if assertions_found == false {
                return Err(YamlParsingError(format!("Test assertions not specified for {}", test_name.unwrap())));
            }

                
            let mut test_assertions_to_push: Vec<TestAssertion> = vec![];

            for test_assertion in test_assertions.iter() {
                let user_says = test_assertion["userSays"].as_str();
                if let None = user_says {
                    return Err(YamlParsingError(format!("Test assertions missing userSays for {}", test_name.unwrap())));
                }
                let user_says = user_says.unwrap();
                let mut bot_responses = vec![];
                let bot_responds_with = test_assertion["botRespondsWith"].as_str();
                if let None = bot_responds_with {
                    let bot_responds_with = test_assertion["botRespondsWith"].as_vec();
                    if let None = bot_responds_with {
                        return Err(YamlParsingError(format!("Test assertions missing botRespondsWith for {}", test_name.unwrap())));
                    } else {
                        let bot_responds_with = bot_responds_with.unwrap();

                        for bot_responds_with_str in bot_responds_with.iter() {
                            bot_responses.push(bot_responds_with_str.as_str().unwrap().to_string());                        
                        }
                    }
                } else {
                    bot_responses.push(bot_responds_with.unwrap().to_string());                        
                }
                test_assertions_to_push.push(TestAssertion::new(user_says.to_string(), bot_responses));

                }
                test_to_push.set_assertions(test_assertions_to_push);
                suite_tests.push(test_to_push);
        } // for

        Ok(
            TestSuite {
                // we can safely unwrap now, None value is not possible here
                suite_spec: TestSuiteSpec::new(name.unwrap().to_string(), suite_type.unwrap(), cred.unwrap().to_string()),
                tests: suite_tests
        }) 
    }
}

pub fn parse (yaml: &str) -> Result<TestSuite, YamlParsingError> {
    TestSuite::new(yaml)
}

#[cfg(test)]
mod tests {
    use super::*;

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
          botRespondsWith: 'Fallback'
        - userSays: 'foo'
          botRespondsWith: ['bar', 'foobar']
";            

const YAML2: &str =
"
suite-spec:
    type: 'DialogFlow'
    cred: '/path/to/cred'
";            

    #[test]
    fn test_parse () {
        let suite =  TestSuite::new(YAML1).unwrap();
        assert_eq!(suite.suite_spec.name, "Express Tracking");
        assert_eq!(suite.tests.len(), 2);
        assert_eq!(suite.tests[0].name, "Welcome intent test");
        
        let mut _desc = &suite.tests[1].desc;
        assert_eq!(_desc.as_ref().unwrap(), "Tests default fallback intent");

        _desc = &suite.tests[0].desc;
        assert_eq!(_desc.as_ref().unwrap(), "Tests default welcome intent");        

        assert_eq!(suite.tests[1].assertions.len(), 2);
        assert_eq!(suite.tests[1].assertions[1].user_says, "foo");
        assert_eq!(suite.tests[1].assertions[1].bot_responds_with, ["bar", "foobar"]);
        assert_eq!(suite.tests[1].assertions[0].bot_responds_with, ["Fallback"]);
    }

    #[test]
    fn test_parse_failed_suite_name_not_found () {
        let result =  TestSuite::new(YAML2);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Suite name not specified".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }
}