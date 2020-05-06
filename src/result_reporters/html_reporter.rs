use crate::yaml_parser::{
    Test,
    TestResult,
    TestAssertionResult
};

#[allow(dead_code)]
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

#[allow(dead_code)]
const OK_SVG: &str =
r#"
    <svg class="bi bi-check-circle text-success" width="1em" height="1em" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
        <path fill-rule="evenodd" d="M15.354 2.646a.5.5 0 010 .708l-7 7a.5.5 0 01-.708 0l-3-3a.5.5 0 11.708-.708L8 9.293l6.646-6.647a.5.5 0 01.708 0z" clip-rule="evenodd"/>
        <path fill-rule="evenodd" d="M8 2.5A5.5 5.5 0 1013.5 8a.5.5 0 011 0 6.5 6.5 0 11-3.25-5.63.5.5 0 11-.5.865A5.472 5.472 0 008 2.5z" clip-rule="evenodd"/>
    </svg>
"#;  

#[allow(dead_code)]
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
    </svg><b>??</b></span>
"#;  

#[allow(dead_code)]
const ACCORDION_ITEM: &str =
r##"
    <div class="card">
    <div class="card-header" id="heading{__test_id__}">
    <h5 class="mb-0">
        <button class="btn btn-link" data-toggle="collapse" data-target="#collapse{__test_id__}" aria-expanded="true" aria-controls="collapse{__test_id__}">
        Test #1 (Hello - track)
            <svg class="bi bi-check-circle text-success" width="1em" height="1em" viewBox="0 0 16 16" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
            <path fill-rule="evenodd" d="M15.354 2.646a.5.5 0 010 .708l-7 7a.5.5 0 01-.708 0l-3-3a.5.5 0 11.708-.708L8 9.293l6.646-6.647a.5.5 0 01.708 0z" clip-rule="evenodd"/>
            <path fill-rule="evenodd" d="M8 2.5A5.5 5.5 0 1013.5 8a.5.5 0 011 0 6.5 6.5 0 11-3.25-5.63.5.5 0 11-.5.865A5.472 5.472 0 008 2.5z" clip-rule="evenodd"/>
            </svg>		  
        </button>
    </h5>
    </div>
    <div id="collapse{__test_id__}" class="collapse" aria-labelledby="heading{__test_id__}" data-parent="#accordion">
        <div class="card-body">
            {__carg_body__}
        </div>
    </div>
    </div>
"##;  

#[allow(dead_code)]
const TEST_ASSERTION_CHECK_ERROR_MSG: &str =
r#"
<b>Assertion post check error:</b></br>
{__err_msg__}
"#;

#[allow(dead_code)]
const TEST_ASSERTION_INTENT_MISMATCH_ERROR_MSG: &str =
r#"
<b>Intent name mismatch:</b></br>
{__err_msg__}
"#;

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
const ASSERTION_CHECK_ROW: &str =
r#"
    <tr>
        <td>{__expression__}</td>
        <td>{__operator__}</td>
        <td>{__value__}</td>
        <td>{__status__}</td>
    </tr>
"#;

#[allow(dead_code)]
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

    pub fn get_test_result_html(test: &Test) -> String {
      
        let test_result_str;
        if let Some(test_result) = &test.test_result {
          match test_result {
            TestResult::Ok => test_result_str = OK_SVG.to_string(),
            TestResult::Ko => test_result_str = KO_SVG.to_string()
          }
        } else {
          test_result_str = UNKNOWN_SVG.to_string();
        }
    
        test_result_str
    }

/*
    pub fn report_test_results(tests: &Vec<Test>) {
        
        let mut test_tables: Vec<Table> = vec![];
  
        for test in tests {
          let mut test_table = Table::new();
          let test_result_str = StdoutResultReporter::get_test_result_str(test);
  
          let test_result = test.get_test_error();
  
          // add header row with test name status string (OK/KO) + potential error message (either intent name mismatch or assertion check error)
          match test_result {
            Some(some_test_result) => {
              match some_test_result {
                TestAssertionResult::KoIntentNameMismatch(err) => {
                  test_table.add_row(
                    row![
                      test.name.clone() + 
                      " - " + 
                      &test_result_str + "\n" + 
                      &err.message
                    ]
                  );
                },
                TestAssertionResult::KoResponseCheckError(err, _) => {
                   test_table.add_row(
                     row![
                       test.name.clone() + 
                       " - " + 
                       &test_result_str + "\n" + 
                       &err.message
                     ]
                   );
                },
                _ => { /* ok will not happen get_test_error is returning none in that case */}
              }
            }
            None => { 
              test_table.add_row(
                row![test.name.clone() + " - " + &test_result_str]
              );
            }        
        } // match test_result
  
        // now add assertion table within second row of master table (test_table)
        let mut test_table_assertions = Table::new();
        test_table_assertions.add_row(row!["Usar says", "Bot responds with", "Intent match status", "Assertion checks", "Raw response"]);
        for assertion in &test.assertions {
  
          match assertion.test_assertion_result.as_ref().unwrap() /* assuming we always have result! */ {
            TestAssertionResult::Ok(_) => {
              test_table_assertions.add_row(
                row![
                  assertion.user_says.clone(), 
                  assertion.bot_responds_with.join("\n"), 
                  StdoutResultReporter::get_ok_str(), 
                  match assertion.response_checks.len() {
                    0 => StdoutResultReporter::get_na_str(),
                    _ => StdoutResultReporter::get_ok_str()
                  }, 
                  "" // if everything is OK do not include backed response in std out report,
                     // it will be collapsed in html report
                     // TBD: other option is to make this configurable
                ]
              );
            },
            TestAssertionResult::KoIntentNameMismatch(err) => {
              test_table_assertions.add_row(
                row![
                  assertion.user_says.clone(), 
                  assertion.bot_responds_with.join("\n"), 
                  StdoutResultReporter::get_ko_str(), 
                  StdoutResultReporter::get_not_executed_str(),
                  err.backend_response.as_ref().unwrap() // TBD: make this configurable!
                ]
              );
              break; // do not continue with any other assertion!
            },
            TestAssertionResult::KoResponseCheckError(err, assertion_check_idx) => {
  
              let mut test_table_assertion_resp_checks = Table::new();
              test_table_assertion_resp_checks.add_row(row!["Expression", "Operator", "Value", "Status"]);
  
              for idx in 0..*assertion_check_idx + 1 {
                let response_check = &assertion.response_checks[idx];
  
                let res_str;
                if idx == *assertion_check_idx {
                  res_str = StdoutResultReporter::get_ko_str()
                } else {
                  res_str = StdoutResultReporter::get_ok_str()
                }
  
                test_table_assertion_resp_checks.add_row(
                  row![
                    response_check.expression,
                    response_check.operator,
                    response_check.value,
                    res_str
                  ]
                );
              }
  
              test_table_assertions.add_row(
                row![
                  assertion.user_says.clone(), 
                  assertion.bot_responds_with.join("\n"), 
                  StdoutResultReporter::get_ok_str(), 
                  test_table_assertion_resp_checks,
                  err.backend_response.as_ref().unwrap() // TBD: make this configurable!
                ]
  
              );
              break; // do not continue with any other assertion!
            },
          }
  
        } // for assertion in test.assertions
        test_table.add_row(row![test_table_assertions]);
        test_tables.push(test_table);      
  
      } // for test in tests 
  
      for table in test_tables {
        table.printstd();
      }
  
    } // report_test_results
*/
}

