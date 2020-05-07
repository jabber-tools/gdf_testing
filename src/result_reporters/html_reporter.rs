use crate::yaml_parser::{
    Test,
    TestAssertionResult
};
use crate::errors::Result;
use std::fs::File;
use std::path::Path;
use std::io::Write;

const MASTER_CONTAINER: &str =
r#"
    <!doctype html>
    <html lang="en">
        <head>
        <!-- Required meta tags -->
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
        <!-- Bootstrap CSS -->
        <link rel="stylesheet" href="https://maxcdn.bootstrapcdn.com/bootstrap/4.0.0/css/bootstrap.min.css" integrity="sha384-Gn5384xqQ1aoWXA+058RXPxPg6fy4IWvTNh0E263XmFcJlSAwiGgFAW/dAiS6JXm" crossorigin="anonymous">
        <title>Dialog Testing Report</title>
        </head>
        <body>
        <!-- Optional JavaScript -->
        <!-- jQuery first, then Popper.js, then Bootstrap JS -->
        <script src="https://code.jquery.com/jquery-3.2.1.slim.min.js" integrity="sha384-KJ3o2DKtIkvYIK3UENzmM7KCkRr/rE9/Qpg6aAZGJwFDMVNA/GpGFF93hXpG5KkN" crossorigin="anonymous"></script>
        <script src="https://cdnjs.cloudflare.com/ajax/libs/popper.js/1.12.9/umd/popper.min.js" integrity="sha384-ApNbgh9B+Y1QKtv3Rn7W3mgPxhU9K/ScQsAP7hUibX39j7fakFPskvXusvfa0b4Q" crossorigin="anonymous"></script>
        <script src="https://maxcdn.bootstrapcdn.com/bootstrap/4.0.0/js/bootstrap.min.js" integrity="sha384-JZR6Spejh4U02d8jOt6vLEHfe/JQGiRRSQQxSfFWpi1MquVdAyjUar5+76PVCmYl" crossorigin="anonymous"></script>
        <div class="p-1"><!-- padding 1 -->
            <div id="accordion">
                {__report_body__}
            </div>
        </div>
        </body>
    </html>  
"#;     

const OK_SVG: &str =
r#"
    <svg class="bi bi-check-circle text-success" width="1em" height="1em" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
        <path fill-rule="evenodd" d="M15.354 2.646a.5.5 0 010 .708l-7 7a.5.5 0 01-.708 0l-3-3a.5.5 0 11.708-.708L8 9.293l6.646-6.647a.5.5 0 01.708 0z" clip-rule="evenodd"/>
        <path fill-rule="evenodd" d="M8 2.5A5.5 5.5 0 1013.5 8a.5.5 0 011 0 6.5 6.5 0 11-3.25-5.63.5.5 0 11-.5.865A5.472 5.472 0 008 2.5z" clip-rule="evenodd"/>
    </svg>
"#;  

const KO_SVG: &str =
r#"
    <svg class="bi bi-x-circle text-danger" width="1em" height="1em" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
        <path fill-rule="evenodd" d="M8 15A7 7 0 108 1a7 7 0 000 14zm0 1A8 8 0 108 0a8 8 0 000 16z" clip-rule="evenodd"/>
        <path fill-rule="evenodd" d="M11.854 4.146a.5.5 0 010 .708l-7 7a.5.5 0 01-.708-.708l7-7a.5.5 0 01.708 0z" clip-rule="evenodd"/>
        <path fill-rule="evenodd" d="M4.146 4.146a.5.5 0 000 .708l7 7a.5.5 0 00.708-.708l-7-7a.5.5 0 00-.708 0z" clip-rule="evenodd"/>
    </svg>
"#;  

#[allow(dead_code)]
const UNKNOWN_SVG: &str =
r#"
    <span><svg class="bi bi-x-circle text-danger" width="1em" height="1em" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
        <path fill-rule="evenodd" d="M8 15A7 7 0 108 1a7 7 0 000 14zm0 1A8 8 0 108 0a8 8 0 000 16z" clip-rule="evenodd"/>
        <path fill-rule="evenodd" d="M11.854 4.146a.5.5 0 010 .708l-7 7a.5.5 0 01-.708-.708l7-7a.5.5 0 01.708 0z" clip-rule="evenodd"/>
        <path fill-rule="evenodd" d="M4.146 4.146a.5.5 0 000 .708l7 7a.5.5 0 00.708-.708l-7-7a.5.5 0 00-.708 0z" clip-rule="evenodd"/>
    </svg><b>(UNKNOWN STATUS)</b></span>
"#;  

const ACCORDION_ITEM: &str =
r##"
    <div class="card">
    <div class="card-header" id="heading{__test_id__}">
    <h5 class="mb-0">
        <button class="btn btn-link" data-toggle="collapse" data-target="#collapse{__test_id__}" aria-expanded="true" aria-controls="collapse{__test_id__}">
        {__test_header__}
        </button>
    </h5>
    </div>
    <div id="collapse{__test_id__}" class="collapse" aria-labelledby="heading{__test_id__}" data-parent="#accordion">
        <div class="card-body">
            {__card_body_test__}
            {__card_body_err_msg__}
        </div>
    </div>
    </div>
"##;  

const TEST_ASSERTION_CHECK_ERROR_MSG: &str =
r#"
<b>Assertion post check error:</b></br>
{__err_msg__}
"#;

const TEST_ASSERTION_INTENT_MISMATCH_ERROR_MSG: &str =
r#"
<b>Intent name mismatch:</b></br>
{__err_msg__}
"#;

const TEST_RESULT_TABLE: &str =
r#"
    <table class="table table-bordered">
    <thead>
    <tr>
        <th colspan="5" scope="col">Test assertions:</th>
    </tr>
    <tr>
        <th scope="col">User says</th>
        <th scope="col">Bot responds with</th>
        <th scope="col">Intent match status</th>
        <th scope="col">Assertion checks</th>
        <th scope="col">Raw response</th>
    </tr>
    </thead>
    <tbody>
        {__assertions__}
    </tbody>
    </table> 
"#;

const ASSERTION_ROW: &str =
r##"
    <tr>
    <td>{__user_says__}</td>
    <td>{__bot_responds_with__}</td>
    <td>
        {__intent_name_match_status__}
    </td>
    <td>
        {__assertion_checks_table__}
    </td>		
    <td>
    <p>
        <a class="btn btn-primary" data-toggle="collapse" href="#collapseExample{__test_id__}{__assertion_id__}" role="button" aria-expanded="false" aria-controls="collapseExample{__test_id__}{__assertion_id__}">
        Show raw response data
        </a>
    </p>
    <div class="collapse" id="collapseExample{__test_id__}{__assertion_id__}">
        <div class="card card-body">
        <pre id="json">
            {__json_raw_response__}	  
        </pre>			
        </div>
    </div>				  
    </td>			  
    </tr>    
"##;

const ASSERTION_CHECK_ROW: &str =
r#"
    <tr>
        <td>{__expression__}</td>
        <td>{__operator__}</td>
        <td>{__value__}</td>
        <td>{__status__}</td>
    </tr>
"#;

const ASSERTION_CHECK_TABLE: &str =
r#"
    <table class="table table-bordered">
    <thead>
        <tr>
            <th scope="col">Expression</th>
            <th scope="col">Operator</th>
            <th scope="col">Value</th>
            <th scope="col">Status</th>
        </tr>
    </thead>
    <tbody>
        {__rows__}
    </tbody>
    </table>
"#;

pub struct HtmlResultReporter;

impl HtmlResultReporter {

    pub fn report_test_results(tests: &Vec<Test>, file_path: &Path) -> Result<()> {
        
        let mut test_tables: Vec<String> = vec![];
  
        for (test_idx, test) in tests.iter().enumerate() {
          
          let test_result = test.get_test_error();
  
          // create header row for each test accordion element
          let mut test_header_html = String::new();
          match test_result {
            Some(some_test_result) => {
              match some_test_result {
                TestAssertionResult::KoIntentNameMismatch(_) => {
                    test_header_html = format!("Test #{} ({}){}", test_idx + 1, test.name.clone(), KO_SVG);

                },
                TestAssertionResult::KoResponseCheckError(_, _) => {
                    test_header_html = format!("Test #{} ({}){}", test_idx + 1, test.name.clone(), KO_SVG);
                },
                _ => { /* ok will not happen get_test_error is returning none in that case */}
              }
            }
            None => { 
                test_header_html = format!("Test #{} ({}){}", test_idx + 1, test.name.clone(), OK_SVG);
            }        
        } // match test_result
  
        // now prepare assertion rows for final assertion table
        let mut test_table_assertions_html: Vec<String> = vec![];
        
        for (assertion_idx, assertion) in test.assertions.iter().enumerate() {
          match assertion.test_assertion_result.as_ref().unwrap() /* assuming we always have result! */ {
            TestAssertionResult::Ok(response) => {

                let mut test_table_assertion_resp_checks:Vec<String> = vec![];

                for response_check in &assertion.response_checks {
    
                  let assertion_check_row =  ASSERTION_CHECK_ROW.to_string()
                  .replace("{__expression__}", &response_check.expression)
                  .replace("{__operator__}", &response_check.operator.to_string())
                  .replace("{__value__}", &response_check.value.to_string())
                  .replace("{__status__}", OK_SVG);
  
                  test_table_assertion_resp_checks.push(assertion_check_row);
                }                

              let assertion_response_check_html = ASSERTION_CHECK_TABLE.to_string().replace("{__rows__}", &test_table_assertion_resp_checks.join(""));  

              let backend_response = String::from("<span>") + response + "</span>";
              let assertion_html = ASSERTION_ROW.to_string()
              .replace("{__user_says__}", &assertion.user_says)
              .replace("{__bot_responds_with__}", &assertion.bot_responds_with.join("</br>"))
              .replace("{__intent_name_match_status__}", OK_SVG)
              .replace("{__assertion_checks_table__}", match assertion.response_checks.len() {
                0 => "<span>No response checks</span>",
                // _ => OK_SVG // do not display assertion response check table when assertion result is OK (same as std out report)
                _ => &assertion_response_check_html // in html report we can go crazy and display full table even for OK assertions
              })
              .replace("{__test_id__}", &test_idx.to_string())
              .replace("{__assertion_id__}", &assertion_idx.to_string())
              .replace("{__json_raw_response__}", &backend_response);
              test_table_assertions_html.push(assertion_html);
            },
            TestAssertionResult::KoIntentNameMismatch(err) => {
              let backend_response = String::from("<span>") + err.backend_response.as_ref().unwrap() + "</span>";
              let assertion_html = ASSERTION_ROW.to_string()
              .replace("{__user_says__}", &assertion.user_says)
              .replace("{__bot_responds_with__}", &assertion.bot_responds_with.join("</br>"))
              .replace("{__intent_name_match_status__}", KO_SVG)
              .replace("{__assertion_checks_table__}", "<span>not executed</span>")
              .replace("{__test_id__}", &test_idx.to_string())
              .replace("{__assertion_id__}", &assertion_idx.to_string())
              .replace("{__json_raw_response__}", &backend_response);
              test_table_assertions_html.push(assertion_html);
              break; // do not continue with any other assertion!
            },
            TestAssertionResult::KoResponseCheckError(err, assertion_check_idx) => {
              let mut test_table_assertion_resp_checks:Vec<String> = vec![];

              for idx in 0..*assertion_check_idx + 1 {
                let response_check = &assertion.response_checks[idx];
  
                let res_str;
                if idx == *assertion_check_idx {
                  res_str = KO_SVG;
                } else {
                  res_str = OK_SVG;
                }
                
                let assertion_check_row =  ASSERTION_CHECK_ROW.to_string()
                .replace("{__expression__}", &response_check.expression)
                .replace("{__operator__}", &response_check.operator.to_string())
                .replace("{__value__}", &response_check.value.to_string())
                .replace("{__status__}", res_str);

                test_table_assertion_resp_checks.push(assertion_check_row);
              }
  
              let backend_response = String::from("<span>") + err.backend_response.as_ref().unwrap() + "</span>";
              let assertion_html = ASSERTION_ROW.to_string()
              .replace("{__user_says__}", &assertion.user_says)
              .replace("{__bot_responds_with__}", &assertion.bot_responds_with.join("</br>"))
              .replace("{__intent_name_match_status__}", KO_SVG)
              .replace("{__assertion_checks_table__}", &ASSERTION_CHECK_TABLE.to_string().replace("{__rows__}", &test_table_assertion_resp_checks.join("")))
              .replace("{__test_id__}", &test_idx.to_string())
              .replace("{__assertion_id__}", &assertion_idx.to_string())
              .replace("{__json_raw_response__}", &backend_response);
              test_table_assertions_html.push(assertion_html);
              break; // do not continue with any other assertion!

            },
          }
  
        } // for assertion in test.assertions
        
        
        // prepare Test error message (if any)
        let test_err_msg;
        match test_result {
            Some(some_test_result) => {
                match some_test_result {
                    TestAssertionResult::KoIntentNameMismatch(err) =>  {test_err_msg = TEST_ASSERTION_INTENT_MISMATCH_ERROR_MSG.to_string().replace("{__err_msg__}", &err.message);},
                    TestAssertionResult::KoResponseCheckError(err, _) => {test_err_msg = TEST_ASSERTION_CHECK_ERROR_MSG.to_string().replace("{__err_msg__}", &err.message);},
                    _ =>  {test_err_msg = String::from("");}, //this will never happen but we need to satisfy compiler
                }
            },
            None =>  {test_err_msg = String::from("");}
        }

        let test_table = TEST_RESULT_TABLE.to_string().replace("{__assertions__}", &test_table_assertions_html.join(""));
        let test_accordion = ACCORDION_ITEM.to_string()
        .replace("{__test_header__}", &test_header_html)
        .replace("{__card_body_test__}", &test_table)
        .replace("{__test_id__}", &test_idx.to_string())
        .replace("{__card_body_err_msg__}",  &test_err_msg);

        test_tables.push(test_accordion);      
  
      } // for test in tests 
  
      let html_report = MASTER_CONTAINER.to_string().replace("{__report_body__}", &test_tables.join(""));
      
      let mut file = File::create(file_path)?;
      file.write_all(html_report.as_bytes())?;
      Ok(())

    } // report_test_results

}

