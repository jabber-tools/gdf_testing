// This module is alternative to yaml_parser. Instaed of coping data from yaml document 
// into heap it strictly uses references to original yaml data. This makes the implementation
// clearer to read (no to_owned() calls, etc.) and more memory efficient 
// (even though this is probably negligible for average size of yaml we will typically work with)
use yaml_rust::Yaml;
use std::fmt;
use yaml_rust::scanner::ScanError;
use std::error::Error;

#[derive(Debug)]
pub struct YamlParsingError(String);

impl fmt::Display for YamlParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "YamlParsingError occurred: {}", &self.0)
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
pub struct TestSuiteSpec<'a> {
    pub name: &'a str,
    pub suite_type: TestSuiteType,
    pub cred: &'a str
}

impl<'a> TestSuiteSpec<'a> {
    fn new(name: &'a str, suite_type: TestSuiteType, cred: &'a str) -> TestSuiteSpec<'a> {
        TestSuiteSpec {
            name,
            suite_type,
            cred
        }
    }
}

#[derive(Debug)]
pub struct TestAssertion<'a> {
    pub user_says: &'a str,
    pub bot_responds_with: Vec<&'a str>
}

impl<'a> TestAssertion<'a> {
    pub fn new(user_says: &'a str, bot_responds_with: Vec<&'a str>) -> TestAssertion<'a> {
        TestAssertion {
            user_says: user_says,
            bot_responds_with: bot_responds_with
        }
    }
}

#[derive(Debug)]
pub struct Test<'a> {
    pub name: &'a str,
    pub desc: Option<&'a str>,
    pub assertions: Vec<TestAssertion<'a>>
}
    
impl<'a> Test<'a> {
    pub fn new(name: &'a str, desc: Option<&'a str>) -> Test<'a> {
        Test {
            name: name,
            desc: desc,
            assertions: vec![]
        }
    }
}

#[derive(Debug)]
pub struct TestSuite<'a> {
    pub suite_spec: TestSuiteSpec<'a>,
    pub tests: Vec<Test<'a>>
}

impl<'a> TestSuite<'a> {

    pub fn new(suite_spec: TestSuiteSpec<'a>, tests: Vec<Test<'a>>) -> TestSuite<'a> {
        TestSuite {
            suite_spec,
            tests
        }
    }

    pub fn from_yaml(yaml: &Yaml) -> Result<TestSuite, YamlParsingError> {

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

        if tests.len() == 0 {
            return Err(YamlParsingError(format!("No tests specified")));
        }

        let mut suite_tests: Vec<Test> = vec![];
        
        for test in tests.iter() {

            let test_name = test["name"].as_str();
            let test_desc = test["desc"].as_str(); //desc is optional
            let test_assertions: Option<&Vec<Yaml>> = test["assertions"].as_vec();
            if let None = test_name {return Err(YamlParsingError(format!("Test name not specified")));}
                
            let mut test_to_push;

            if let None = test_desc {
                test_to_push = Test::new(test_name.unwrap(), None);
            } else {
                test_to_push = Test::new(test_name.unwrap(), Some(test_desc.unwrap()));
            }

            
            if let None = test_assertions { 
                return Err(YamlParsingError(format!("Test assertions not specified for {}", test_name.unwrap())));
            } else if let Some(vec_of_yaml_ref) = test_assertions {
                if vec_of_yaml_ref.len() == 0 { 
                    return Err(YamlParsingError(format!("Test assertions not specified for {}", test_name.unwrap())));
                }
            } else {
                // code will never get here (Option can be either None or Some, nothing else) adding else branch just for sure and explicitness
                panic!("unexpected value found while processing test_assertions");
            }
            
            let mut test_assertions_to_push: Vec<TestAssertion> = vec![];

            // safe to unwrap test_assertions now
            for test_assertion in test_assertions.unwrap().iter() {
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
                            let bot_responds_with_str = bot_responds_with_str.as_str().unwrap();
                            if bot_responds_with_str.trim()  == "" {
                                return Err(YamlParsingError(format!("Test assertions botRespondsWith cannot be empty for {}", test_name.unwrap())));
                            }
                            bot_responses.push(bot_responds_with_str);                        
                        }
                    }
                } else {
                    let _bot_responds_with = bot_responds_with.unwrap();
                    if _bot_responds_with.trim()  == "" {
                        return Err(YamlParsingError(format!("Test assertions botRespondsWith cannot be empty for {}", test_name.unwrap())));
                    }
                    bot_responses.push(_bot_responds_with);                        
                }
                test_assertions_to_push.push(TestAssertion::new(user_says, bot_responses));

                }
                test_to_push.assertions.extend(test_assertions_to_push);
                suite_tests.push(test_to_push);
        } // for

        Ok(
            TestSuite {
                // we can safely unwrap now, None value is not possible here
                suite_spec: TestSuiteSpec::new(name.unwrap(), suite_type.unwrap(), cred.unwrap()),
                tests: suite_tests
        }) 
    }
}

pub fn parse (docs: &Vec<Yaml>) -> Result<TestSuite, YamlParsingError> {
    TestSuite::from_yaml(&docs[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use yaml_rust::YamlLoader;

    #[test]
    fn compose_test_suite () {
        let assertion1 = TestAssertion::new("Hi", vec!["Welcome","Welcome2"]);
        let assertion2 = TestAssertion::new("whats up?", vec!["Smalltalk|Whats up"]);
        
        let mut test1 = Test::new("Test1", None);
        test1.assertions = vec![assertion1, assertion2];
        
        let suite_spec = TestSuiteSpec::new("Express Tracking", TestSuiteType::DialogFlow, "/path/to/cred");

        let suite = TestSuite::new(suite_spec, vec![test1]);

        assert_eq!(suite.suite_spec.name, "Express Tracking");
        assert_eq!(suite.tests.len(), 1);
        assert_eq!(suite.tests[0].name, "Test1");
        
        assert_eq!(suite.tests[0].assertions.len(), 2);
        assert_eq!(suite.tests[0].assertions[1].user_says, "whats up?");
        assert_eq!(suite.tests[0].assertions[1].bot_responds_with, ["Smalltalk|Whats up"]);
        assert_eq!(suite.tests[0].assertions[0].bot_responds_with, ["Welcome", "Welcome2"]);        

    }

    #[test]
    fn test_from_yaml_str () -> Result<(), YamlParsingError> {

        const YAML: &str =
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

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let suite =  TestSuite::from_yaml(yaml).unwrap();
        assert_eq!(suite.suite_spec.name, "Express Tracking");
        assert_eq!(suite.tests.len(), 2);
        assert_eq!(suite.tests[0].name, "Welcome intent test");
        
        let mut _desc = &suite.tests[1].desc;
        assert_eq!(_desc.unwrap(), "Tests default fallback intent");

        _desc = &suite.tests[0].desc;
        assert_eq!(_desc.unwrap(), "Tests default welcome intent");        

        assert_eq!(suite.tests[1].assertions.len(), 2);
        assert_eq!(suite.tests[1].assertions[1].user_says, "foo");
        assert_eq!(suite.tests[1].assertions[1].bot_responds_with, ["bar", "foobar"]);
        assert_eq!(suite.tests[1].assertions[0].bot_responds_with, ["Fallback"]);
        Ok(())
    }

    #[test]
    fn test_parse_failed_suite_name_not_found () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            type: "DialogFlow"
            cred: "/path/to/cred"
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Suite name not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }

    #[test]
    fn test_parse_failed_unknown_suite_type () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "SomeNonsense"
            cred: "/path/to/cred"
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Unknown suite type found: SomeNonsense".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    

    #[test]
    fn test_parse_failed_suite_type_not_specified () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            cred: "/path/to/cred"
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Suite type not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    

    #[test]
    fn test_parse_failed_credentials_not_specified () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Suite credentials not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }        

    #[test]
    fn test_parse_no_tests_1 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "No tests specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }      

    #[test]
    fn test_parse_no_tests_2 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "No tests specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }          

    #[test]
    fn test_parse_name_not_specified () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - desc: 'Tests default welcome intent'
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
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test name not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    
    
    #[test]
    fn test_parse_assertions_not_specified_1 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - name: "Welcome intent test"
              desc: "Tests default welcome intent"
            - name: "Default fallback intent"
              desc: "Tests default fallback intent"
              assertions:
                - userSays: "wtf"
                  botRespondsWith: "Fallback"
                - userSays: "foo"
                  botRespondsWith: ["bar", "foobar"]
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions not specified for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    
    
    #[test]
    fn test_parse_assertions_not_specified_2 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - name: "Welcome intent test"
              desc: "Tests default welcome intent"
              assertions:
            - name: "Default fallback intent"
              desc: "Tests default fallback intent"
              assertions:
                - userSays: "wtf"
                  botRespondsWith: "Fallback"
                - userSays: "foo"
                  botRespondsWith: ["bar", "foobar"]
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions not specified for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }       

    #[test]
    fn test_parse_assertions_missing_user_says () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - name: "Welcome intent test"
              desc: "Tests default welcome intent"
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: ['Welcome']
            - name: "Default fallback intent"
              desc: "Tests default fallback intent"
              assertions:
                - userSays123: "wtf"
                  botRespondsWith: "Fallback"
                - userSays: "foo"
                  botRespondsWith: ["bar", "foobar"]
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions missing userSays for Default fallback intent".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }       

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_1 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - name: "Welcome intent test"
              desc: "Tests default welcome intent"
              assertions:
                - userSays: 'Hello'
            - name: "Default fallback intent"
              desc: "Tests default fallback intent"
              assertions:
                - userSays: "wtf"
                  botRespondsWith: "Fallback"
                - userSays: "foo"
                  botRespondsWith: ["bar", "foobar"]
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions missing botRespondsWith for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }         

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_2 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - name: "Welcome intent test"
              desc: "Tests default welcome intent"
              assertions:
                - userSays: 'Hello'
                  botRespondsWith:
            - name: "Default fallback intent"
              desc: "Tests default fallback intent"
              assertions:
                - userSays: "wtf"
                  botRespondsWith: "Fallback"
                - userSays: "foo"
                  botRespondsWith: ["bar", "foobar"]
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions missing botRespondsWith for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }          

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_3 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - name: "Welcome intent test"
              desc: "Tests default welcome intent"
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: ''
            - name: "Default fallback intent"
              desc: "Tests default fallback intent"
              assertions:
                - userSays: "wtf"
                  botRespondsWith: "Fallback"
                - userSays: "foo"
                  botRespondsWith: ["bar", "foobar"]
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions botRespondsWith cannot be empty for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }        

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_4 () -> Result<(), YamlParsingError> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
            - name: "Welcome intent test"
              desc: "Tests default welcome intent"
              assertions:
                - userSays: 'Hello'
                  botRespondsWith: ['']
            - name: "Default fallback intent"
              desc: "Tests default fallback intent"
              assertions:
                - userSays: "wtf"
                  botRespondsWith: "Fallback"
                - userSays: "foo"
                  botRespondsWith: ["bar", "foobar"]
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions botRespondsWith cannot be empty for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }   
}