use crate::errors::{Result};

use crate::yaml_parser::{
    TestResult,
    TestAssertion, 
    TestAssertionResult,
};
use crate::suite_executor::TestSuiteExecutor;

mod gdf_executor;
mod vap_executor;
pub use vap_executor::VAPTestExecutor;
pub use gdf_executor::GDFDefaultTestExecutor;

pub trait TestExecutor {
    // helper abstaract methods so that we can use default implementations for next_assertion_details/execute_next_assertion
    fn move_to_next_assertion(&mut self);
    fn move_behind_last_assertion(&mut self);
    fn get_assertions(&self) -> &Vec<TestAssertion>;
    fn set_test_result(&mut self, test_result: TestResult);
    fn set_test_assertion_result(&mut self, test_assertion_result: TestAssertionResult);
    fn get_next_assertion_no(&self) -> usize;
    fn send_test_results(&self) -> Result<()>;
    //
    // core abstract method to be provided for every test executor //
    //
    fn invoke_nlp(&self, assertion: &TestAssertion) -> Result<String>;

    // these default implementation hardcode default flow for convenience
    // every test executor can than focus on invoke_nlp only
    fn next_assertion_details(&self) -> Option<&TestAssertion> {
        let next_assertion_no = self.get_next_assertion_no();
        let assertions = self.get_assertions();

        if next_assertion_no >= assertions.len() {
            let _ = self.send_test_results();
            None
        } else {
            let assertion_to_execute = &assertions[next_assertion_no];
            Some(assertion_to_execute)
        }
    }

    fn execute_next_assertion(&mut self) -> Option<()> {

        let next_assertion_no = self.get_next_assertion_no();
        let assertions = self.get_assertions();

        if next_assertion_no >= assertions.len() {
            self.set_test_result(TestResult::Ok);
            let _ = self.send_test_results();
            return None;
        } else {
            let assertion_to_execute = &assertions[next_assertion_no];

            let assertion_response = self.invoke_nlp(assertion_to_execute);

            if let Err(intent_mismatch_error) = assertion_response {
                // if intent name does not match expected value do not continue
                self.set_test_assertion_result(TestAssertionResult::KoIntentNameMismatch(intent_mismatch_error));
                self.set_test_result(TestResult::Ko);
                self.move_behind_last_assertion();
                let _ = self.send_test_results();
                return None;
            } 

            // otherwise try to run assertion response checks
            let assertion_response = assertion_response.unwrap();

            for response_check in &assertion_to_execute.response_checks {
                let response_check_result = TestSuiteExecutor::process_assertion_response_check(response_check, &assertion_response);
    
                if let Err(some_response_check_error) = response_check_result {
                    self.set_test_assertion_result(TestAssertionResult::KoResponseCheckError(some_response_check_error));
                    self.set_test_result(TestResult::Ko);
                    self.move_behind_last_assertion();
                    let _ = self.send_test_results();
                    return None;
                }
            } 
            
            self.set_test_assertion_result(TestAssertionResult::Ok(assertion_response));
            self.move_to_next_assertion();
            return Some(());                

        }
    }      
}