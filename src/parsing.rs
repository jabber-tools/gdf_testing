use yaml_rust;

pub mod yaml {
    pub fn parse (yaml: &str) {
        let docs = yaml_rust::YamlLoader::load_from_str(yaml).unwrap();
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
    #[test]
    fn test_parse () {
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
                - botRespondsWith:
                    - Welcome
        ";    
        parse(yaml);
    }
}