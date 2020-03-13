use gdf_testing;

fn main() {
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


    gdf_testing::parse_yaml(yaml);
}