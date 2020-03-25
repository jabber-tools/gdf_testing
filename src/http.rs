use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, Header, Algorithm, EncodingKey, DecodingKey};
use std::time::SystemTime;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64
}

fn pem_file_to_str(file_name: &str) -> Option<String> {
    let result = fs::read_to_string(file_name);
    match result {
        // replace \n literals (i.e. "\\n") with real end line character (i.e. "\n")!
        Ok(file_str) => Some(file_str.replace("\\n", "\n")),
        _ => None
    }
}

 fn pem_to_encoding_key(file_name: &str) -> Option<EncodingKey> {
    let file_str_opt = fs::read_to_string(file_name).ok();
    match file_str_opt {
        Some(file_str) => {
            // replace \n literals (i.e. "\\n") with real end line character (i.e. "\n")!
            let file_str = file_str.replace("\\n", "\n");
            EncodingKey::from_rsa_pem(file_str.into_bytes().as_slice()).ok()
        },
        _ => None
    }
 } 


 fn pem_to_decoding_key<'a>(file_bytes: &'a Vec<u8>) -> Option<DecodingKey<'a>> {
    // DecodingKey::from_rsa_pem(&file_bytes[..]).ok()
    let res = DecodingKey::from_rsa_pem(&file_bytes[..]);
    match res {
        Ok(x) => Some(x),
        Err(err) => {
            println!("error is {}", err);
            None
        }
    }
 } 

// see https://github.com/Keats/jsonwebtoken
fn new_token(client_email: &str, priv_key_file: &str) -> Option<String> {
    let _now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let claims = Claims {
        iss: client_email.to_owned(),
        scope: "https://www.googleapis.com/auth/cloud-platform".to_owned(),
        aud: "https://www.googleapis.com/oauth2/v4/token".to_owned(),
        exp: _now + 3600,
        iat: _now
    };

    // RS256 - encrypting with private key
    let encoding_key = pem_to_encoding_key(priv_key_file);
    if encoding_key == None {
        return None;
    }
    
    let priv_key = &encoding_key.unwrap();
    encode(&Header::new(Algorithm::RS256), &claims, priv_key).ok()
}


#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, Validation, errors::ErrorKind };

    // this is integration test for which you need DialogFlow agent credentials (i.e. private key + associated service account email) and respective public key
    // get the credentials from google cloud console (json file)
    // put private_key filed of json file into separate pem file (privkey.pem). use it as second argument in new_token call (see below "./src/testdata/privkey.pem")
    // extract client_email json attribute and use it as first argument in new_token (below df-client-admin-access@express-cs-common-dev.iam.gserviceaccount.com)
    // json field client_x509_cert_url holds link from where certificate can be downloaded
    // this file actually holds two certificates
    // convert them to pub keys using commands (cert.crt/cert2.crt holds pem certificate retrieved from url as per above):
    // openssl x509 -pubkey -noout -in cert.crt  > pubkey.pem
    // openssl x509 -pubkey -noout -in cert2.crt  > pubkey2.pem
    // for validation you need to use certificate where issued to/issued by is equal to the value client_id from json file with credentials!
    #[test]
    #[ignore]
    fn test_new_token() {
        let token = new_token("df-client-admin-access@express-cs-common-dev.iam.gserviceaccount.com", "./src/testdata/privkey.pem");
        match token {
            // Some(_token) => assert!(false, format!("token {}", _token)),
            Some(_token) => {

                // using uncorrect public key should result in InvalidSignature error
                let cert_str = pem_file_to_str("./src/testdata/pubkey.pem").unwrap();
                let cert_str_bytes = cert_str.into_bytes();
                let dec_key = pem_to_decoding_key(&cert_str_bytes).unwrap();
                let decoded_token = decode::<Claims>(&_token, &dec_key, &Validation::new(Algorithm::RS256));
                match decoded_token {
                    Err(err) =>  {
                        match err.kind() {
                            ErrorKind::InvalidSignature => assert!(true),
                            _ => assert!(false, "expected InvalidSignature error, got different error instead")
                        }
                    },
                    _ => assert!(false, "expected InvalidSignature error, got result instead")
                }

                // using correct public key we should be able to decode the token and examine claims values
                let cert_str = pem_file_to_str("./src/testdata/pubkey2.pem").unwrap();
                let cert_str_bytes = cert_str.into_bytes();
                let dec_key = pem_to_decoding_key(&cert_str_bytes).unwrap();
                let decoded_token = decode::<Claims>(&_token, &dec_key, &Validation::new(Algorithm::RS256)).unwrap();

                let claims = decoded_token.claims;
                assert_eq!(claims.iss, "df-client-admin-access@express-cs-common-dev.iam.gserviceaccount.com");
                assert_eq!(claims.aud, "https://www.googleapis.com/oauth2/v4/token");
                assert_eq!(claims.scope, "https://www.googleapis.com/auth/cloud-platform");
                assert_eq!(claims.exp - claims.iat, 3600);
            },
            _ => assert!(false, "no token generated!")
        }
    }
}    