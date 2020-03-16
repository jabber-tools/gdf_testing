use gdf_testing;
use std::error::Error;
use yaml_rust::YamlLoader;

#[allow(unused_must_use)]
fn main() -> Result<(), Box<dyn Error>> {
    println!("parsing yaml");

    let yaml =
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
    ";   
  
    let docs = YamlLoader::load_from_str(yaml)?;
    let suite = gdf_testing::parse(&docs)?;
    
    println!("got the suite {:#?}", suite);
    Ok(())
}