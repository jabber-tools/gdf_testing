mod parsing;

pub fn parse_yaml(yaml: &str) {
    parsing::yaml::parse(yaml);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_yaml_parse () {
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
        parse_yaml(yaml);
    }
}
