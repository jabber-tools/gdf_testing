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

    pub fn new(suite_spec: TestSuiteSpec, tests: Vec<Test>) -> Self {
        TestSuite {
            suite_spec,
            tests
        }
    }

    pub fn from_yaml_str(yaml: &str) -> Result<TestSuite, YamlParsingError> {
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

        if tests.len() == 0 {
            return Err(YamlParsingError(format!("No tests specified")));
        }

        let mut suite_tests: Vec<Test> = vec![];

        for test in tests.iter() {

            let test_name = test["name"].as_str();
            let test_desc = test["desc"].as_str(); //desc is optional
            let test_assertions = test["assertions"].as_vec();
            if let None = test_name {return Err(YamlParsingError(format!("Test name not specified")));}
                
            let mut test_to_push;

            if let None = test_desc {
                test_to_push = Test::new(test_name.unwrap().to_owned(), None);
            } else {
                test_to_push = Test::new(test_name.unwrap().to_owned(), Some(test_desc.unwrap().to_string()));
            }

            // cannot just shaddow the variable in else section (i.e. test_assertions_vec = test_assertions.unwrap())
            // since compiler will benot able to infer the type properly. instead we must explicitly cast test_assertions_vec
            let mut test_assertions_vec: &Vec<Yaml> = &vec![];
            let mut assertions_found = true;
            if let None = test_assertions { 
                assertions_found = false; 
            } else {
                test_assertions_vec = test_assertions.unwrap();
                if test_assertions_vec.len() == 0 { 
                    assertions_found = false; 
                }
            }
            
            if assertions_found == false {
                return Err(YamlParsingError(format!("Test assertions not specified for {}", test_name.unwrap())));
            }
            
            let mut test_assertions_to_push: Vec<TestAssertion> = vec![];

            for test_assertion in test_assertions_vec.iter() {
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
                            bot_responses.push(bot_responds_with_str.to_string());                        
                        }
                    }
                } else {
                    let _bot_responds_with = bot_responds_with.unwrap();
                    if _bot_responds_with.trim()  == "" {
                        return Err(YamlParsingError(format!("Test assertions botRespondsWith cannot be empty for {}", test_name.unwrap())));
                    }
                    bot_responses.push(_bot_responds_with.to_string());                        
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
    TestSuite::from_yaml_str(yaml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_test_suite () {
        let assertion1 = TestAssertion::new("Hi".to_string(), vec!["Welcome".to_string(),"Welcome2".to_string()]);
        let assertion2 = TestAssertion::new("whats up?".to_string(), vec!["Smalltalk|Whats up".to_string()]);
        
        let mut test1 = Test::new("Test1".to_string(), None);
        test1.set_assertions(vec![assertion1, assertion2]);
        
        let suite_spec = TestSuiteSpec::new("Express Tracking".to_string(), TestSuiteType::DialogFlow, "/path/to/cred".to_string());

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
    fn test_from_yaml_str () {

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

        let suite =  TestSuite::from_yaml_str(YAML).unwrap();
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

        const YAML: &str =
        r#"
        suite-spec:
            type: "DialogFlow"
            cred: "/path/to/cred"
        "#;                    

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Suite name not specified".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }

    #[test]
    fn test_parse_failed_unknown_suite_type () {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "SomeNonsense"
            cred: "/path/to/cred"
        "#;                    

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Unknown suite type found: SomeNonsense".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }    

    #[test]
    fn test_parse_failed_suite_type_not_specified () {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            cred: "/path/to/cred"
        "#;                    

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Suite type not specified".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }    

    #[test]
    fn test_parse_failed_credentials_not_specified () {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
        "#;                    

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Suite credentials not specified".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }        

    #[test]
    fn test_parse_no_tests_1 () {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        "#;                    

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "No tests specified".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }      

    #[test]
    fn test_parse_no_tests_2 () {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            cred: "/path/to/cred"
        tests:
        "#;                    

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "No tests specified".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }          

    #[test]
    fn test_parse_name_not_specified () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test name not specified".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }    
    
    #[test]
    fn test_parse_assertions_not_specified_1 () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions not specified for Welcome intent test".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }    
    
    #[test]
    fn test_parse_assertions_not_specified_2 () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions not specified for Welcome intent test".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }       

    #[test]
    fn test_parse_assertions_missing_user_says () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions missing userSays for Default fallback intent".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }       

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_1 () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions missing botRespondsWith for Welcome intent test".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }         

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_2 () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions missing botRespondsWith for Welcome intent test".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }          

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_3 () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions botRespondsWith cannot be empty for Welcome intent test".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }        

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_4 () {

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

        let result =  TestSuite::from_yaml_str(YAML);
        match result {
            Err(e) => {
                assert_eq!(e.0, "Test assertions botRespondsWith cannot be empty for Welcome intent test".to_string());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
    }   
}