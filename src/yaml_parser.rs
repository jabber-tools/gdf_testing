use yaml_rust::Yaml;
use crate::errors::{Result, ErrorKind, new_error_from, Error};
use std::collections::HashMap;

fn yaml_error(message: String) -> Error {
    new_error_from(ErrorKind::YamlParsingError(message))
}  

#[derive(Debug, Clone)]
pub enum TestSuiteType {
    DialogFlow,
    DHLVAP
}

#[derive(Debug)]
pub struct TestSuiteSpec {
    pub name: String,
    pub suite_type: TestSuiteType,
    pub config: HashMap<String, String>
}

impl Clone for TestSuiteSpec {
    fn clone(&self) -> TestSuiteSpec {
        TestSuiteSpec {
            name: self.name.clone(),
            suite_type: self.suite_type.clone(),
            config: self.config.clone()
        }
    }
}

impl TestSuiteSpec {
    fn new(name: String, suite_type: TestSuiteType, config: HashMap<String, String>) -> TestSuiteSpec {
        TestSuiteSpec {
            name,
            suite_type,
            config
        }
    }
}

#[derive(Debug)]
pub struct TestAssertion {
    pub user_says: String,
    pub bot_responds_with: Vec<String>,
    pub response_checks: Vec<TestAssertionResponseCheck>,
    pub test_assertion_result: Option<TestAssertionResult>,
}

#[derive(Debug, Clone)]
pub enum TestAssertionResult {
    Ok(String), // contains NLP provider response
    KoIntentNameMismatch(Error), // error contains both error description and NLP provider response (see Error.backend_response)
    KoResponseCheckError(Error)
}

impl Clone for TestAssertion {
    fn clone(&self) -> TestAssertion {
        TestAssertion {
            user_says: self.user_says.clone(),
            bot_responds_with: self.bot_responds_with.clone(),
            response_checks: self.response_checks.clone(),
            test_assertion_result: self.test_assertion_result.clone()
        }
    }
}

impl TestAssertion {
    pub fn new(user_says: String, bot_responds_with: Vec<String>, response_checks: Vec<TestAssertionResponseCheck>) -> TestAssertion {
        TestAssertion {
            user_says,
            bot_responds_with,
            response_checks,
            test_assertion_result: None
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TestAssertionResponseCheckOperator {
    Equals,
    NotEquals,
    JsonEquals,
    Includes,
    Length
}

#[derive(Debug, PartialEq)]
pub enum TestAssertionResponseCheckValue {
    StrVal(String),
    NumVal(f64),
    BoolVal(bool),
}

impl Clone for TestAssertionResponseCheckValue {
    fn clone(&self) -> TestAssertionResponseCheckValue {
        match self {
            TestAssertionResponseCheckValue::BoolVal(bool_val) => TestAssertionResponseCheckValue::BoolVal(bool_val.clone()),
            TestAssertionResponseCheckValue::StrVal(str_val) => TestAssertionResponseCheckValue::StrVal(str_val.clone()),
            TestAssertionResponseCheckValue::NumVal(num_val) => TestAssertionResponseCheckValue::NumVal(num_val.clone())
        }
    }
}

#[derive(Debug)]
pub struct TestAssertionResponseCheck {
    pub expression: String,
    pub operator: TestAssertionResponseCheckOperator,
    pub value: TestAssertionResponseCheckValue
}

impl Clone for TestAssertionResponseCheck {
    fn clone(&self) -> TestAssertionResponseCheck {
        TestAssertionResponseCheck {
            expression: self.expression.clone(),
            operator: self.operator.clone(),
            value: self.value.clone()
        }
    }
}

impl TestAssertionResponseCheck {
    pub fn new(expression: String, operator: TestAssertionResponseCheckOperator, value: TestAssertionResponseCheckValue) -> Self {
        TestAssertionResponseCheck {
            expression,
            operator,
            value
        }
    }
}

#[derive(Debug, Clone)]
pub enum TestResult {
    Ok,
    Ko
}


#[derive(Debug)]
pub struct Test {
    pub name: String,
    pub desc: Option<String>,
    pub assertions: Vec<TestAssertion>,
    pub execution_id: Option<usize>,
    pub test_result: Option<TestResult>,
}

impl Clone for Test {
    fn clone(&self) -> Test {
        Test {
            name: self.name.clone(),
            desc: self.desc.clone(),
            assertions: self.assertions.clone(),
            execution_id: self.execution_id.clone(),
            test_result: self.test_result.clone()
        }
    }
}
    
impl Test {
    pub fn new(name: String, desc: Option<String>) -> Test {
        Test {
            name: name,
            desc: desc,
            assertions: vec![],
            execution_id: None,
            test_result: None
        }
    }

    pub fn get_test_error(&self) -> Option<&TestAssertionResult> {
        for assertion in &self.assertions {
            if let Some(assertion_result) = &assertion.test_assertion_result {
                match assertion_result {
                    TestAssertionResult::KoIntentNameMismatch(_) |
                    TestAssertionResult::KoResponseCheckError(_) => return Some(assertion_result),
                    _  => {}, 
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct TestSuite {
    pub suite_spec: TestSuiteSpec,
    pub tests: Vec<Test>
}

impl Clone for TestSuite {
    fn clone(&self) -> TestSuite {
        TestSuite {
            suite_spec: self.suite_spec.clone(),
            tests: self.tests.clone()
        }
    }
}

impl TestSuite {

    pub fn new(suite_spec: TestSuiteSpec, tests: Vec<Test>) -> TestSuite {
        TestSuite {
            suite_spec,
            tests
        }
    }


    fn retrieve_response_checks (yaml: &Yaml, test_name: &str, assertion_name: &str) -> Result<Vec<TestAssertionResponseCheck>> {
        let response_checks = &yaml["responseChecks"];
        let response_checks = response_checks.as_vec();
        if let None = response_checks {
            return Ok(vec![])
        }

        let mut test_assertion_response_check_vec = vec![];

        for response_check in response_checks.unwrap().iter() {
            let expression = response_check["expression"].as_str();
            let operator = response_check["operator"].as_str();
            let value = &response_check["value"];

            if let None = expression {
                return Err(yaml_error(format!("expression name not specified for. test '{}', assertion: '{}'", test_name, assertion_name)))
            }

            let expression = expression.unwrap();

            if let None = operator {
                return Err(yaml_error(format!("operator name not specified. test: '{}', assertion: '{}', expression: '{}'", test_name, assertion_name, expression)))
            }
            
            let _operator =  match operator.unwrap() {
                "equals" => TestAssertionResponseCheckOperator::Equals,
                "!equals" => TestAssertionResponseCheckOperator::NotEquals,
                "jsonequals" => TestAssertionResponseCheckOperator::JsonEquals,
                "includes" => TestAssertionResponseCheckOperator::Includes,
                "length" => TestAssertionResponseCheckOperator::Length,
                _ =>  return Err(yaml_error(format!("unsupported operator({}). test: '{}', assertion: '{}', expression: '{}'. Supported values: equals, !equals', 'jsonequals', 'includes', 'length'", operator.unwrap(),  test_name, assertion_name, expression)))
            };

            // see https://github.com/chyh1990/yaml-rust/blob/master/src/yaml.rs
            let _value = match &*value {
                Yaml::Integer(ival) => TestAssertionResponseCheckValue::NumVal(*ival as f64),
                Yaml::Real(fval) => TestAssertionResponseCheckValue::NumVal(fval.parse::<f64>().unwrap()),
                Yaml::String(sval) => TestAssertionResponseCheckValue::StrVal(sval.to_string()),
                Yaml::Boolean(bval) => TestAssertionResponseCheckValue::BoolVal(*bval),
                _ => return Err(yaml_error(format!("unsupported value specified. test: '{}', assertion: '{}', expression: '{}'", test_name, assertion_name, expression)))
            };

            test_assertion_response_check_vec.push(TestAssertionResponseCheck::new(expression.to_string(), _operator, _value));
        }

        Ok(test_assertion_response_check_vec)
    }

    fn retrieve_suite_config (yaml: &Yaml) -> Option<HashMap<String, String>> {
        let config = yaml["suite-spec"]["config"].as_vec();

        let mut config_map: HashMap<String, String> = HashMap::new();

        if let Some(_config) = config {
            for _iter in _config {
                
                let config_item = _iter.as_hash()?;
                for (k, v) in config_item {
                    let key = k.as_str();
                    let val = v.as_str();
    
                    if let Some(_key) = key {
                        if let Some(_val) = val {
                            config_map.insert(
                                _key.to_owned(),
                                _val.to_owned()
                            );                
                        }
                    }                    

                }
            }
        }

        if config_map.len() > 0 {
            Some(config_map)
        } else {
            None
        }
    }

    pub fn from_yaml(yaml: &Yaml) -> Result<TestSuite> {

        let name: Option<&str> = yaml["suite-spec"]["name"].as_str();
        if let None = name {
            return Err(yaml_error(format!("Suite name not specified")));
        }

        let suite_type: Option<&str> = yaml["suite-spec"]["type"].as_str();
        let suite_type: Option<TestSuiteType> = match suite_type {
            Some("DialogFlow") => Some(TestSuiteType::DialogFlow),
            Some("DHLVAP") => Some(TestSuiteType::DHLVAP),
            Some(unknown) =>  return Err(yaml_error(format!("Unknown suite type found: {}", unknown))),
            None => return Err(yaml_error(String::from("Suite type not specified")))
        };

        let suite_config = TestSuite::retrieve_suite_config(yaml);
        if let None = suite_config {
            return Err(yaml_error(format!("Suite config not specified")));
        }
        let suite_config = suite_config.unwrap();
            
        let tests = yaml["tests"].as_vec();
        if let None = tests {
            return Err(yaml_error(format!("No tests specified")));
        }

        let tests = tests.unwrap();

        if tests.len() == 0 {
            return Err(yaml_error(format!("No tests specified")));
        }

        let mut suite_tests: Vec<Test> = vec![];
        
        for test in tests.iter() {

            let test_name = test["name"].as_str();
            let test_desc = test["desc"].as_str(); //desc is optional
            let test_assertions: Option<&Vec<Yaml>> = test["assertions"].as_vec();
            if let None = test_name {return Err(yaml_error(format!("Test name not specified")));}
                
            let mut test_to_push;

            if let None = test_desc {
                test_to_push = Test::new(test_name.unwrap().to_string(), None);
            } else {
                test_to_push = Test::new(test_name.unwrap().to_string(), Some(test_desc.unwrap().to_string()));
            }

            
            if let None = test_assertions { 
                return Err(yaml_error(format!("Test assertions not specified for {}", test_name.unwrap())));
            } else if let Some(vec_of_yaml_ref) = test_assertions {
                if vec_of_yaml_ref.len() == 0 { 
                    return Err(yaml_error(format!("Test assertions not specified for {}", test_name.unwrap())));
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
                    return Err(yaml_error(format!("Test assertions missing userSays for {}", test_name.unwrap())));
                }
                let user_says = user_says.unwrap().to_string();
                let mut bot_responses:Vec<String> = vec![];
                let bot_responds_with = test_assertion["botRespondsWith"].as_str();
                if let None = bot_responds_with {
                    let bot_responds_with = test_assertion["botRespondsWith"].as_vec();
                    if let None = bot_responds_with {
                        return Err(yaml_error(format!("Test assertions missing botRespondsWith for {}", test_name.unwrap())));
                    } else {
                        let bot_responds_with = bot_responds_with.unwrap();

                        for bot_responds_with_str in bot_responds_with.iter() {
                            let bot_responds_with_str = bot_responds_with_str.as_str().unwrap().to_string();
                            if bot_responds_with_str.trim()  == "" {
                                return Err(yaml_error(format!("Test assertions botRespondsWith cannot be empty for {}", test_name.unwrap())));
                            }
                            bot_responses.push(bot_responds_with_str);                        
                        }
                    }
                } else {
                    let _bot_responds_with = bot_responds_with.unwrap().to_string();
                    if _bot_responds_with.trim()  == "" {
                        return Err(yaml_error(format!("Test assertions botRespondsWith cannot be empty for {}", test_name.unwrap())));
                    }
                    bot_responses.push(_bot_responds_with);                        
                }
                let response_checks = TestSuite::retrieve_response_checks(test_assertion, test_name.unwrap(), &user_says)?;
                test_assertions_to_push.push(TestAssertion::new(user_says, bot_responses, response_checks));

                }
                test_to_push.assertions.extend(test_assertions_to_push);
                suite_tests.push(test_to_push);
        } // for

        Ok(
            TestSuite {
                // we can safely unwrap now, None value is not possible here
                suite_spec: TestSuiteSpec::new(name.unwrap().to_string(), suite_type.unwrap(), suite_config),
                tests: suite_tests
        }) 
    }
}

pub fn parse (docs: &Vec<Yaml>) -> Result<TestSuite> {
    TestSuite::from_yaml(&docs[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use yaml_rust::YamlLoader;
    use assert_json_diff::assert_json_eq;
    use crate::json_parser::*;

    // convenience function for testing
    fn unwrap_yaml_parsing_error(error: Error) -> String {
        match *error.kind {
            ErrorKind::YamlParsingError(error_str) => error_str,
            _ => panic!("Expected YamlParsingError, got different error type!")
        }
    }    

    #[test]
    fn compose_test_suite () {
        let assertion1 = TestAssertion::new("Hi".to_string(), vec!["Welcome".to_string(),"Welcome2".to_string()], vec![]);
        let assertion2 = TestAssertion::new("whats up?".to_string(), vec!["Smalltalk|Whats up".to_string()], vec![]);
        
        let mut test1 = Test::new("Test1".to_string(), None);
        test1.assertions = vec![assertion1, assertion2];
        
        let mut config_map = HashMap::new();
        config_map.insert(
            "credentials_file".to_string(),
            "/Users/abezecny/adam/WORK/_DEV/Rust/gdf_testing/src/testdata/credentials.json".to_string()
        );

        let suite_spec = TestSuiteSpec::new("Express Tracking".to_string(), TestSuiteType::DialogFlow, config_map);

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
    fn test_from_yaml_str () -> Result<()> {

        const YAML: &str =
        "
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/path/to/cred'
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
        
        let mut _desc = suite.tests[1].desc.as_ref();
        assert_eq!(_desc.unwrap(), "Tests default fallback intent");

        _desc = suite.tests[0].desc.as_ref();
        assert_eq!(_desc.unwrap(), "Tests default welcome intent");        

        assert_eq!(suite.tests[1].assertions.len(), 2);
        assert_eq!(suite.tests[1].assertions[1].user_says, "foo");
        assert_eq!(suite.tests[1].assertions[1].bot_responds_with, ["bar", "foobar"]);
        assert_eq!(suite.tests[1].assertions[0].bot_responds_with, ["Fallback"]);
        Ok(())
    }

    #[test]
    fn test_parse_failed_suite_name_not_found () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);
        match result {
            Err(e) => {
                assert_eq!(unwrap_yaml_parsing_error(e), "Suite name not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }

    #[test]
    fn test_parse_failed_unknown_suite_type () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "SomeNonsense"
            config: 
              - credentials_file: '/path/to/cred'
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(unwrap_yaml_parsing_error(e), "Unknown suite type found: SomeNonsense".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    

    #[test]
    fn test_parse_failed_suite_type_not_specified () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            config: 
              - credentials_file: '/path/to/cred'
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(unwrap_yaml_parsing_error(e), "Suite type not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    

    #[test]
    fn test_parse_failed_suite_config_not_specified () -> Result<()> {

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
                assert_eq!(unwrap_yaml_parsing_error(e), "Suite config not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }        

    #[test]
    fn test_parse_no_tests_1 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(unwrap_yaml_parsing_error(e), "No tests specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }      

    #[test]
    fn test_parse_no_tests_2 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
        tests:
        "#;                    

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);

        match result {
            Err(e) => {
                assert_eq!(unwrap_yaml_parsing_error(e), "No tests specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }          

    #[test]
    fn test_parse_name_not_specified () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test name not specified".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    
    
    #[test]
    fn test_parse_assertions_not_specified_1 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test assertions not specified for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }    
    
    #[test]
    fn test_parse_assertions_not_specified_2 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test assertions not specified for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }       

    #[test]
    fn test_parse_assertions_missing_user_says () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test assertions missing userSays for Default fallback intent".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }       

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_1 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test assertions missing botRespondsWith for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }         

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_2 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test assertions missing botRespondsWith for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }          

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_3 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test assertions botRespondsWith cannot be empty for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }        

    #[test]
    fn test_parse_assertions_missing_bot_responds_with_4 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: "Express Tracking"
            type: "DialogFlow"
            config: 
              - credentials_file: '/path/to/cred'
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
                assert_eq!(unwrap_yaml_parsing_error(e), "Test assertions botRespondsWith cannot be empty for Welcome intent test".to_owned());
            },
            _ => {panic!("error was supposed to be thrown!")}
        }
        Ok(())
    }   

    #[test]
    fn test_assertion_extension_1 () -> Result<()> {

        const YAML: &str =
        r#"
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/path/to/cred'
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
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: '!equals'
                      value: 'action.foo'
                    - expression: 'queryResult.outputContexts[0].name'
                      operator: 'includes'
                      value: 'bar_prompt'
                    - expression: 'queryResult.outputContexts[0].lifespanCount'
                      operator: 'equals'
                      value: 2
                    - expression: 'queryResult.outputContexts'
                      operator: 'length'
                      value: 1
                    - expression: 'queryResult.allRequiredParamsPresent'
                      operator: 'equals'
                      value: false
                - userSays: 'bar'
                  botRespondsWith: ['foo']
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: 'equals'
                      value: 'action.foobar'
                    - expression: 'queryResult.fulfillmentText'
                      operator: 'includes'
                      value: 'foo bar'
                    - expression: 'queryResult.outputContexts'
                      operator: 'jsonequals'
                      value: |
                        {"stuff": {
                            "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
                            "lifespanCount": 1
                        }}                      
        "#; 

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let suite =  TestSuite::from_yaml(yaml).unwrap();
        
        assert_eq!(suite.tests[1].assertions[1].user_says, "foo");
        assert_eq!(suite.tests[1].assertions[1].response_checks.len(), 5);
        assert_eq!(suite.tests[1].assertions[1].response_checks[0].expression, "queryResult.action");
        assert_eq!(suite.tests[1].assertions[1].response_checks[0].operator, TestAssertionResponseCheckOperator::NotEquals);
        assert_eq!(suite.tests[1].assertions[1].response_checks[0].value, TestAssertionResponseCheckValue::StrVal("action.foo".to_string()));

        // thisl will not work because of whitespaces differences, we need to compare jsons in normalized way!
        /* assert_eq!(suite.tests[1].assertions[2].response_checks[2].value, TestAssertionResponseCheckValue::StrVal(r#"{
            "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
            "lifespanCount": 1
        }"#)); */
        let value_expected: serde_json::value::Value = serde_json::from_str(r#"{
            "name": "projects/express-cs-dummy/agent/sessions/98fe9b3d-fa99-53cf-062c-d20cfab9f123/contexts/tracking_prompt",
            "lifespanCount": 1
        }"#).unwrap();

        match &suite.tests[1].assertions[2].response_checks[2].value {
            TestAssertionResponseCheckValue::StrVal(str_val) => {

                let parser = JsonParser::new(&str_val);

                // TBD: we need to find way how search/jmespath can access whole json
                // if not possible real implementation must wrap the content by stuff placeholder implicitly
                // so that users don't have to do it
                let search_result = parser.search("stuff").unwrap();
                let value_real = JsonParser::extract_as_object(&search_result);
                
                if let Some(_value_real) = value_real {
                    assert_json_eq!(serde_json::json!(*_value_real), value_expected);
                } else {
                    assert!(false, "None value returned by extract_as_object");
                }
                
            },
            _ => assert!(false, "string value expected in asertion response check value!")
        }

        Ok(())
    }    

    #[test]
    fn test_assertion_extension_no_expression () -> Result<()> {

        const YAML: &str =
        "
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/path/to/cred'
        tests:
            - name: 'Default fallback intent'
              desc: 'Tests default fallback intent'
              assertions:
                - userSays: 'foo'
                  botRespondsWith: 'bar'
                  responseChecks:
                    - expressionXX: 'queryResult.action'
                      operator: '!equals'
                      value: 'action.foo'
        "; 

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);
        match result {
            Err(e) => assert_eq!(unwrap_yaml_parsing_error(e), "expression name not specified for. test 'Default fallback intent', assertion: 'foo'"),
            _ => panic!("error was supposed to be thrown!")
        }
        Ok(())
    }        

    #[test]
    fn test_assertion_extension_no_operator () -> Result<()> {

        const YAML: &str =
        "
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/path/to/cred'
        tests:
            - name: 'Default fallback intent'
              desc: 'Tests default fallback intent'
              assertions:
                - userSays: 'foo'
                  botRespondsWith: 'bar'
                  responseChecks:
                    - expression: 'queryResult.action'
                      operatorXX: '!equals'
                      value: 'action.foo'
        "; 

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);
        match result {
            Err(e) => assert_eq!(unwrap_yaml_parsing_error(e), "operator name not specified. test: 'Default fallback intent', assertion: 'foo', expression: 'queryResult.action'"),
            _ => panic!("error was supposed to be thrown!")
        }
        Ok(())
    }        

    #[test]
    fn test_assertion_extension_invalid_operator () -> Result<()> {

        const YAML: &str =
        "
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/path/to/cred'
        tests:
            - name: 'Default fallback intent'
              desc: 'Tests default fallback intent'
              assertions:
                - userSays: 'foo'
                  botRespondsWith: 'bar'
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: 'not in'
                      value: 'action.foo'
        "; 

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);
        match result {
            Err(e) => assert_eq!(unwrap_yaml_parsing_error(e), "unsupported operator(not in). test: 'Default fallback intent', assertion: 'foo', expression: 'queryResult.action'. Supported values: equals, !equals', 'jsonequals', 'includes', 'length'"),
            _ => panic!("error was supposed to be thrown!")
        }
        Ok(())
    }          

    #[test]
    fn test_assertion_extension_invalid_value () -> Result<()> {

        const YAML: &str =
        "
        suite-spec:
            name: 'Express Tracking'
            type: 'DialogFlow'
            config: 
              - credentials_file: '/path/to/cred'
        tests:
            - name: 'Default fallback intent'
              desc: 'Tests default fallback intent'
              assertions:
                - userSays: 'foo'
                  botRespondsWith: 'bar'
                  responseChecks:
                    - expression: 'queryResult.action'
                      operator: 'equals'
                      value:
                        - foo: 'bar'
                          bar: 'foo'
        "; 

        let docs = YamlLoader::load_from_str(YAML)?;
        let yaml: &Yaml = &docs[0];

        let result =  TestSuite::from_yaml(yaml);
        match result {
            Err(e) => assert_eq!(unwrap_yaml_parsing_error(e), "unsupported value specified. test: 'Default fallback intent', assertion: 'foo', expression: 'queryResult.action'"),
            _ => panic!("error was supposed to be thrown!")
        }
        Ok(())
    }        
}